use anyhow::{anyhow, Context, Result};
use clap::Parser;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use lru::LruCache;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    io::{BufReader, Read, Seek},
    num::NonZeroUsize,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use chd::metadata::{KnownMetadata, Metadata, MetadataTag};
use chd::Chd;

/// Expose 2048-byte ISO stream from CD CHDs and passthrough from DVD CHDs.
const TTL: Duration = Duration::from_secs(1);
const CD_FRAME_2352: usize = 2352;

/// Flags / CLI
#[derive(Parser, Debug)]
#[command(name = "chd2iso-fuse", version, about = "Present CHD images as ISO files via FUSE")]
struct Args {
    /// Source directory containing *.chd files
    #[arg(short = 's', long = "source", value_name = "DIR")]
    source_dir: PathBuf,

    /// Mountpoint
    #[arg(short = 'm', long = "mount", value_name = "DIR")]
    mountpoint: PathBuf,

    /// Allow other users to access the mount (requires user_allow_other in /etc/fuse.conf)
    #[arg(long = "allow-other", default_value_t = false)]
    allow_other: bool,

    /// Max in-memory cache entries (frames) across all files
    #[arg(long = "cache-hunks", default_value_t = 256)]
    cache_hunks: usize,

    /// Soft cap for cache memory usage (bytes)
    #[arg(long = "cache-bytes", default_value_t = 256 * 1024 * 1024)]
    cache_bytes: usize,

    /// Permit exporting Mode2/Form2 payloads as raw 2324-byte sectors (exposed as "Name (Form2).bin")
    #[arg(long = "cd-allow-form2", default_value_t = false)]
    cd_allow_form2: bool,

    /// Verbose logging
    #[arg(long = "verbose", default_value_t = false)]
    verbose: bool,
}

#[derive(Clone, Debug)]
enum BackingKind {
    /// DVD (or generic 2048 units): direct 2048 sector passthrough
    Dvd2048,
    /// CD-style (2352 frames) -> user-data view with offsets & mapping
    Cd2352 {
        first_data_lba: u64,
        payload_kind: CdPayloadKind,
        track_frames: Option<u64>, // if known via metadata
    },
    /// Raw/unrecognized, default to 2048 passthrough (rare/fallback)
    Raw2048,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CdPayloadKind {
    Mode1_2048,      // MODE1 (2048)
    Mode2Form1_2048, // MODE2/Form1 (2048)
    Mode2Form2_2324, // MODE2/Form2 (2324) — only if user opts in; exposed as .bin
}

#[derive(Clone, Debug)]
struct IndexEntry {
    ino: u64,
    name: String,     // displayed filename (.iso or (Form2).bin)
    chd_path: PathBuf,
    kind: BackingKind,
    iso_size: u64, // size exposed to userspace
}

struct Handle {
    file_id: u64,
    chd_path: PathBuf,
}

struct FsState {
    args: Args,
    entries: Vec<IndexEntry>,
    // fh -> Handle
    handles: HashMap<u64, Handle>,
    next_fh: u64,
    // LRU cache for frames: key=(file_id, frame_index) -> 2352 bytes
    frame_cache: LruCache<(u64, u64), Vec<u8>>,
    approx_cache_bytes: usize,
}

impl FsState {
    fn new(args: Args) -> Result<Self> {
        let cache_cap =
            NonZeroUsize::new(args.cache_hunks).unwrap_or(NonZeroUsize::new(64).unwrap());
        Ok(Self {
            entries: Vec::new(),
            handles: HashMap::new(),
            next_fh: 1,
            frame_cache: LruCache::new(cache_cap),
            approx_cache_bytes: 0,
            args,
        })
    }

    fn build_index(&mut self) -> Result<()> {
        let dir = &self.args.source_dir;
        let mut tmp: Vec<IndexEntry> = Vec::new();

        for ent in fs::read_dir(dir).with_context(|| format!("reading {:?}", dir))? {
            let ent = ent?;
            let path = ent.path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("chd"))
                != Some(true)
            {
                continue;
            }

            match self.build_index_entry(&path) {
                Ok(Some((name, kind, size))) => {
                    tmp.push(IndexEntry {
                        ino: 0, // assign later
                        name,
                        chd_path: path.clone(),
                        kind,
                        iso_size: size,
                    });
                }
                Ok(None) => {
                    // intentionally hidden (e.g., Form2 without opt-in)
                }
                Err(e) => {
                    error!("Skipping {:?}: {}", path, e);
                }
            }
        }

        // stable sort
        tmp.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        // assign inode numbers deterministically
        for (i, e) in tmp.iter_mut().enumerate() {
            e.ino = (i as u64) + 2; // root=1
        }
        self.entries = tmp;
        Ok(())
    }

    fn build_index_entry(&self, chd_path: &Path) -> Result<Option<(String, BackingKind, u64)>> {
        // Open CHD with an owned BufReader so its type is `BufReader<File>`
        let f = File::open(chd_path)?;
        let mut chd = Chd::open(BufReader::new(f), None)?;

        let hdr = chd.header();
        let unit_bytes = hdr.unit_bytes() as usize;
        let logical_bytes = hdr.logical_bytes();

        let stem = chd_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // DVD or generic 2048-unit image?
        if unit_bytes == 2048 {
            let iso_size = logical_bytes;
            let name = format!("{stem}.iso");
            return Ok(Some((name, BackingKind::Dvd2048, iso_size)));
        }

        // CD-style (2352 frames)
        if unit_bytes == 2352 {
            let total_frames = logical_bytes / 2352;

            // Prefer metadata TOC – pass a fresh reader matching the CHD's reader type
            if let Some((first_lba, payload, track_frames)) = {
                let mut rf = BufReader::new(File::open(chd_path)?);
                parse_cd_toc_from_metadata(&mut chd, &mut rf, self.args.cd_allow_form2)?
            } {
                let (per_sector, name) = match payload {
                    CdPayloadKind::Mode1_2048 | CdPayloadKind::Mode2Form1_2048 => {
                        (2048u64, format!("{stem}.iso"))
                    }
                    CdPayloadKind::Mode2Form2_2324 => {
                        if self.args.cd_allow_form2 {
                            (2324u64, format!("{stem} (Form2).bin"))
                        } else {
                            return Ok(None);
                        }
                    }
                };

                let frames = track_frames.unwrap_or(total_frames - first_lba);
                let iso_size = frames * per_sector;
                let kind = BackingKind::Cd2352 {
                    first_data_lba: first_lba,
                    payload_kind: payload,
                    track_frames: track_frames.map(|v| v as u64),
                };
                return Ok(Some((name, kind, iso_size)));
            }

            // Fallback: quick scan
            let (first_lba, payload) =
                quick_scan_first_data(&mut chd, total_frames, self.args.cd_allow_form2)?;
            let (per_sector, name) = match payload {
                CdPayloadKind::Mode1_2048 | CdPayloadKind::Mode2Form1_2048 => {
                    (2048u64, format!("{stem}.iso"))
                }
                CdPayloadKind::Mode2Form2_2324 => {
                    if self.args.cd_allow_form2 {
                        (2324u64, format!("{stem} (Form2).bin"))
                    } else {
                        return Ok(None);
                    }
                }
            };

            let iso_size = (total_frames - first_lba) * per_sector;
            let kind = BackingKind::Cd2352 {
                first_data_lba: first_lba,
                payload_kind: payload,
                track_frames: None,
            };
            return Ok(Some((name, kind, iso_size)));
        }

        // Fallback: treat as 2048
        let name = format!("{stem}.iso");
        Ok(Some((name, BackingKind::Raw2048, logical_bytes)))
    }

    fn alloc_fh(&mut self) -> u64 {
        let fh = self.next_fh;
        self.next_fh += 1;
        fh
    }

    fn read_iso_from_cd(
        &mut self,
        file_id: u64,
        path: &Path,
        start_frame: u64,
        payload_kind: CdPayloadKind,
        offset: u64,
        size: u32,
        max_len: u64, // clamp to file size
        reply: ReplyData,
    ) {
        let per_sector = match payload_kind {
            CdPayloadKind::Mode1_2048 | CdPayloadKind::Mode2Form1_2048 => 2048usize,
            CdPayloadKind::Mode2Form2_2324 => 2324usize,
        };
        let payload_start = match payload_kind {
            CdPayloadKind::Mode1_2048 => 16usize,
            CdPayloadKind::Mode2Form1_2048 => 24usize,
            CdPayloadKind::Mode2Form2_2324 => 24usize,
        };

        if offset >= max_len || size == 0 {
            reply.data(&[]);
            return;
        }
        let end = (offset.saturating_add(size as u64)).min(max_len);

        let mut want = (end - offset) as usize;
        let mut out = Vec::with_capacity(want);
        let mut cur_iso_sector = (offset as usize) / per_sector;
        let mut cur_in_sector_off = (offset as usize) % per_sector;

        while want > 0 {
            let frame_idx = start_frame + cur_iso_sector as u64;
            let sec = match self.get_cd_frame(file_id, path, frame_idx) {
                Ok(v) => v,
                Err(e) => {
                    error!("frame read error: {:?}", e);
                    reply.error(libc::EIO);
                    return;
                }
            };

            let payload = &sec[payload_start..payload_start + per_sector];
            let avail = per_sector - cur_in_sector_off;
            let take = avail.min(want);

            out.extend_from_slice(&payload[cur_in_sector_off..cur_in_sector_off + take]);
            want -= take;
            cur_iso_sector += 1;
            cur_in_sector_off = 0;
        }

        reply.data(&out);
    }

    /// Return the 2352-byte frame; uses/updates the LRU. Returns an owned Vec to avoid aliasing.
    fn get_cd_frame(&mut self, file_id: u64, path: &Path, frame_index: u64) -> Result<Vec<u8>> {
        if let Some(buf) = self.frame_cache.get(&(file_id, frame_index)) {
            return Ok(buf.clone());
        }

        // Decode the frame (via its containing hunk)
        let f = File::open(path)?;
        let mut chd = Chd::open(BufReader::new(f), None)?;

        let hunk_bytes = chd.header().hunk_size() as usize;
        let frames_per_hunk = hunk_bytes / CD_FRAME_2352;
        if frames_per_hunk == 0 {
            return Err(anyhow!("invalid hunk size for CD"));
        }
        let hunk_index = (frame_index as usize) / frames_per_hunk;
        let frame_in_hunk = (frame_index as usize) % frames_per_hunk;

        let mut hunk_buf = chd.get_hunksized_buffer();
        let mut cmp_buf = Vec::new();

        let mut hk = chd.hunk(hunk_index as u32)?;
        hk.read_hunk_in(&mut cmp_buf, &mut hunk_buf)?;

        let frame_off = frame_in_hunk * CD_FRAME_2352;
        let owned = hunk_buf[frame_off..frame_off + CD_FRAME_2352].to_vec();

        self.approx_cache_bytes += owned.len();
        self.evict_cache_if_needed();
        self.frame_cache.put((file_id, frame_index), owned.clone());

        Ok(owned)
    }

    fn evict_cache_if_needed(&mut self) {
        while self.approx_cache_bytes > self.args.cache_bytes {
            if let Some((_k, v)) = self.frame_cache.pop_lru() {
                self.approx_cache_bytes = self.approx_cache_bytes.saturating_sub(v.len());
            } else {
                break;
            }
        }
    }
}

/// Parse CD TOC from CHD metadata (CHTR/CHT2). Returns (first_data_lba, payload_kind, frames_in_track).
fn parse_cd_toc_from_metadata<R: Read + Seek>(
    chd: &mut Chd<R>,
    file: &mut R,
    allow_form2: bool,
) -> Result<Option<(u64, CdPayloadKind, Option<u64>)>> {
    let mut tracks: Vec<TrackInfo> = Vec::new();

    let mut it = chd.metadata_refs();
    while let Some(mref) = it.next() {
        let md: Metadata = mref.read(file)?;
        let tag = md.metatag;
        // Only track entries
        if tag != KnownMetadata::CdRomTrack.metatag()
            && tag != KnownMetadata::CdRomTrack2.metatag()
        {
            continue;
        }
        let s = String::from_utf8_lossy(&md.value).to_string();
        if let Some(ti) = parse_track_line(&s) {
            tracks.push(ti);
        }
    }

    if tracks.is_empty() {
        return Ok(None);
    }

    tracks.sort_by_key(|t| t.number);

    // Compute absolute LBA across pregaps/frames/postgaps as we walk
    let mut lba: u64 = 0;
    for t in &tracks {
        lba += t.pregap as u64;

        let payload = match t.kind {
            TrackKind::Audio => None,
            TrackKind::Mode1 => Some(CdPayloadKind::Mode1_2048),
            TrackKind::Mode2Form1 => Some(CdPayloadKind::Mode2Form1_2048),
            TrackKind::Mode2Form2 => {
                if allow_form2 {
                    Some(CdPayloadKind::Mode2Form2_2324)
                } else {
                    None
                }
            }
            TrackKind::Mode2Raw => None,
        };

        if let Some(pk) = payload {
            // First data track found
            let frames_in_track = t.frames as u64;
            return Ok(Some((lba, pk, Some(frames_in_track))));
        }

        lba += t.frames as u64;
        lba += t.postgap as u64;
    }

    Ok(None)
}

#[derive(Debug, Clone)]
struct TrackInfo {
    number: u32,
    kind: TrackKind,
    frames: u32,
    pregap: u32,
    postgap: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackKind {
    Audio,
    Mode1,
    Mode2Form1,
    Mode2Form2,
    Mode2Raw,
}

fn parse_track_line(s: &str) -> Option<TrackInfo> {
    // Example:
    // "TRACK:1 TYPE:MODE1 SUBTYPE:NONE FRAMES:26888 PREGAP:0 PGTYPE:MODE1 PGSUB:RW_RAW POSTGAP:0\0"
    let mut number = None;
    let mut frames = 0u32;
    let mut pregap = 0u32;
    let mut postgap = 0u32;
    let mut kind = None::<TrackKind>;

    for tok in s.split(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == ',') {
        if tok.is_empty() {
            continue;
        }
        if let Some((k, v)) = tok.split_once(':') {
            match k {
                "TRACK" => number = v.parse().ok(),
                "FRAMES" => frames = v.parse().unwrap_or(0),
                "PREGAP" => pregap = v.parse().unwrap_or(0),
                "POSTGAP" => postgap = v.parse().unwrap_or(0),
                "TYPE" => {
                    kind = Some(match v {
                        "MODE1" => TrackKind::Mode1,
                        "MODE2/2048" | "MODE2_FORM1" => TrackKind::Mode2Form1,
                        "MODE2/2324" | "MODE2_FORM2" => TrackKind::Mode2Form2,
                        "MODE2/2352" | "MODE2_RAW" | "CDI/2352" => TrackKind::Mode2Raw,
                        "AUDIO" => TrackKind::Audio,
                        other => {
                            if other.starts_with("MODE2") && other.contains("2048") {
                                TrackKind::Mode2Form1
                            } else if other.starts_with("MODE2") && other.contains("2324") {
                                TrackKind::Mode2Form2
                            } else {
                                TrackKind::Audio
                            }
                        }
                    })
                }
                _ => {}
            }
        }
    }

    Some(TrackInfo {
        number: number?,
        kind: kind?,
        frames,
        pregap,
        postgap,
    })
}

/// Fallback when metadata is missing: scan early frames to find a data sector.
fn quick_scan_first_data<R: Read + Seek>(
    chd: &mut Chd<R>,
    total_frames: u64,
    allow_form2: bool,
) -> Result<(u64, CdPayloadKind)> {
    let scan_limit = total_frames.min(2000);
    let mut cmp = Vec::new();
    let mut hbuf = chd.get_hunksized_buffer();
    let frames_per_hunk = (chd.header().hunk_size() as usize) / CD_FRAME_2352;

    let mut frame: u64 = 0;
    while frame < scan_limit {
        let hunk_index = (frame as usize) / frames_per_hunk;
        let frame_in_hunk = (frame as usize) % frames_per_hunk;

        let mut hk = chd.hunk(hunk_index as u32)?;
        hk.read_hunk_in(&mut cmp, &mut hbuf)?;

        let base = frame_in_hunk * CD_FRAME_2352;
        let sec = &hbuf[base..base + CD_FRAME_2352];

        let mode = sec[0x0F];
        if mode == 0x01 {
            return Ok((frame, CdPayloadKind::Mode1_2048));
        } else if mode == 0x02 {
            if allow_form2 {
                return Ok((frame, CdPayloadKind::Mode2Form2_2324));
            } else {
                return Ok((frame, CdPayloadKind::Mode2Form1_2048));
            }
        }

        frame += 1;
    }

    // Default
    Ok((0, CdPayloadKind::Mode1_2048))
}

impl Filesystem for FsState {
    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        if let Some(e) = self.entries.iter().find(|e| e.name == name_str) {
            let attr = file_attr_for(e).unwrap_or_else(|_| default_file_attr(e));
            reply.entry(&TTL, &attr, 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if ino == 1 {
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 1,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::UNIX_EPOCH,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: unsafe { libc::geteuid() },
                gid: unsafe { libc::getegid() },
                rdev: 0,
                flags: 0,
                blksize: 4096,
            };
            reply.attr(&TTL, &attr);
            return;
        }
        if let Some(e) = self.entries.iter().find(|e| e.ino == ino) {
            match file_attr_for(e) {
                Ok(attr) => reply.attr(&TTL, &attr),
                Err(_) => reply.error(libc::EIO),
            }
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(libc::ENOTDIR);
            return;
        }

        let mut idx = offset;
        if idx == 0 {
            let _ = reply.add(1, 1, FileType::Directory, ".");
            let _ = reply.add(1, 2, FileType::Directory, "..");
            idx = 2;
        }
        let mut ent_idx = 3i64;
        for e in &self.entries {
            if ent_idx <= idx {
                ent_idx += 1;
                continue;
            }
            if reply.add(e.ino, ent_idx, FileType::RegularFile, e.name.as_str()) {
                break;
            }
            ent_idx += 1;
        }
        reply.ok();
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        // Copy out fields first (avoid aliasing borrows later)
        let (file_id, chd_path) = if let Some(e) = self.entries.iter().find(|e| e.ino == ino) {
            (e.ino, e.chd_path.clone())
        } else {
            reply.error(libc::ENOENT);
            return;
        };

        if File::open(&chd_path).is_err() {
            reply.error(libc::EIO);
            return;
        }

        let fh = self.alloc_fh();
        self.handles.insert(
            fh,
            Handle {
                file_id,
                chd_path,
            },
        );
        reply.opened(fh, 0);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.handles.remove(&fh);
        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let ent = match self.entries.iter().find(|e| e.ino == ino) {
            Some(e) => e.clone(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        if size == 0 || offset < 0 {
            reply.data(&[]);
            return;
        }

        // Copy out handle fields to avoid immutable borrow conflict
        let (file_id, chd_path) = match self.handles.get(&fh) {
            Some(h) => (h.file_id, h.chd_path.clone()),
            None => {
                reply.error(libc::EBADF);
                return;
            }
        };

        match ent.kind {
            BackingKind::Dvd2048 | BackingKind::Raw2048 => {
                // Map the logical byte range (2048 units) via hunks
                let start = offset as u64;
                let end = (start + size as u64).min(ent.iso_size);
                let to_read = (end - start) as usize;

                let f = match File::open(&chd_path) {
                    Ok(f) => f,
                    Err(_) => {
                        reply.error(libc::EIO);
                        return;
                    }
                };
                let mut chd = match Chd::open(BufReader::new(f), None) {
                    Ok(c) => c,
                    Err(_) => {
                        reply.error(libc::EIO);
                        return;
                    }
                };

                let hunk_size = chd.header().hunk_size() as u64;
                let mut buf = vec![0u8; to_read];
                let mut out_off = 0usize;
                let mut left = to_read as u64;
                let mut pos = start;

                while left > 0 {
                    let hunk_idx = (pos / hunk_size) as u32;
                    let in_hunk_off = (pos % hunk_size) as usize;
                    let take = ((hunk_size as usize) - in_hunk_off).min(left as usize);

                    let mut hunk_buf = chd.get_hunksized_buffer();
                    let mut cmp = Vec::new();
                    let mut hk = match chd.hunk(hunk_idx) {
                        Ok(h) => h,
                        Err(_) => {
                            reply.error(libc::EIO);
                            return;
                        }
                    };
                    if hk.read_hunk_in(&mut cmp, &mut hunk_buf).is_err() {
                        reply.error(libc::EIO);
                        return;
                    }

                    buf[out_off..out_off + take]
                        .copy_from_slice(&hunk_buf[in_hunk_off..in_hunk_off + take]);

                    out_off += take;
                    left -= take as u64;
                    pos += take as u64;
                }

                reply.data(&buf);
            }
            BackingKind::Cd2352 {
                first_data_lba,
                payload_kind,
                track_frames,
            } => {
                // ISO view begins at first_data_lba; clamp length to track_frames if known
                let per_sector = match payload_kind {
                    CdPayloadKind::Mode1_2048 | CdPayloadKind::Mode2Form1_2048 => 2048u64,
                    CdPayloadKind::Mode2Form2_2324 => 2324u64,
                };
                let max_len = if let Some(fr) = track_frames {
                    fr * per_sector
                } else {
                    ent.iso_size // already (total_frames - first_lba) * sector
                };
                self.read_iso_from_cd(
                    file_id,
                    &chd_path,
                    first_data_lba,
                    payload_kind,
                    offset as u64,
                    size,
                    max_len,
                    reply,
                );
            }
        }
    }
}

fn default_file_attr(e: &IndexEntry) -> FileAttr {
    FileAttr {
        ino: e.ino,
        size: e.iso_size,
        blocks: (e.iso_size + 511) / 512,
        atime: SystemTime::now(),
        mtime: SystemTime::now(),
        ctime: SystemTime::now(),
        crtime: SystemTime::UNIX_EPOCH,
        kind: FileType::RegularFile,
        perm: 0o444,
        nlink: 1,
        uid: unsafe { libc::geteuid() },
        gid: unsafe { libc::getegid() },
        rdev: 0,
        flags: 0,
        blksize: 4096,
    }
}

fn file_attr_for(e: &IndexEntry) -> Result<FileAttr> {
    let meta = e.chd_path.metadata()?;
    Ok(FileAttr {
        ino: e.ino,
        size: e.iso_size,
        blocks: (e.iso_size + 511) / 512,
        atime: SystemTime::now(),
        mtime: SystemTime::UNIX_EPOCH + Duration::from_secs(meta.mtime() as u64),
        ctime: SystemTime::UNIX_EPOCH + Duration::from_secs(meta.ctime() as u64),
        crtime: SystemTime::UNIX_EPOCH,
        kind: FileType::RegularFile,
        perm: 0o444,
        nlink: 1,
        uid: meta.uid(),
        gid: meta.gid(),
        rdev: 0,
        flags: 0,
        blksize: 4096,
    })
}

fn main() -> Result<()> {
    let args = Args::parse();

    let filter = if args.verbose {
        EnvFilter::new("info")
    } else {
        EnvFilter::new("warn")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Pre-check mountpoint to avoid EIO on mount
    if args.mountpoint.metadata().is_err() {
        return Err(anyhow!(
            "Mountpoint {:?} does not exist or is not accessible",
            args.mountpoint
        ));
    }

    let mut fs = FsState::new(args)?;
    fs.build_index()?;

    let mut options = vec![
        MountOption::FSName("chd2iso".into()),
        MountOption::RO,
        MountOption::AutoUnmount,
        MountOption::DefaultPermissions,
    ];
    if fs.args.allow_other {
        options.push(MountOption::AllowOther);
    }

    info!(
        "mounting {:?} -> {:?} (entries: {})",
        fs.args.source_dir, fs.args.mountpoint, fs.entries.len()
    );

    // capture before move
    let mountpoint = fs.args.mountpoint.clone();
    fuser::mount2(fs, &mountpoint, &options).map_err(|e| anyhow!("mount failed: {e}"))
}
