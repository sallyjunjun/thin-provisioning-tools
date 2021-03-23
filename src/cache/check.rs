use anyhow::anyhow;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::collections::*;

use crate::io_engine::{AsyncIoEngine, IoEngine, SyncIoEngine};
use crate::cache::hint::*;
use crate::cache::mapping::*;
use crate::cache::superblock::*;
use crate::pdata::array_walker::*;

//------------------------------------------

const MAX_CONCURRENT_IO: u32 = 1024;

//------------------------------------------

struct CheckMappingVisitor {
    allowed_flags: u32,
    seen_oblocks: Arc<Mutex<BTreeSet<u64>>>,
}

impl CheckMappingVisitor {
    fn new(metadata_version: u32) -> CheckMappingVisitor {
        let mut flags: u32 = MappingFlags::Valid as u32;
        if metadata_version == 1 {
            flags |= MappingFlags::Dirty as u32;
        }
        CheckMappingVisitor {
            allowed_flags: flags,
            seen_oblocks: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    fn seen_oblock(&self, b: u64) -> bool {
        let seen_oblocks = self.seen_oblocks.lock().unwrap();
        return seen_oblocks.contains(&b);
    }

    fn record_oblock(&self, b: u64) {
        let mut seen_oblocks = self.seen_oblocks.lock().unwrap();
        seen_oblocks.insert(b);
    }

    // FIXME: is it possible to validate flags at an early phase?
    // e.g., move to ctor of Mapping?
    fn has_unknown_flags(&self, m: &Mapping) -> bool {
        return (m.flags & self.allowed_flags) != 0;
    }
}

impl ArrayBlockVisitor<Mapping> for CheckMappingVisitor {
    fn visit(&self, _index: u64, m: Mapping) -> anyhow::Result<()> {
        if !m.is_valid() {
            return Ok(());
        }

        if self.seen_oblock(m.oblock) {
            return Err(anyhow!("origin block already mapped"));
        }

        self.record_oblock(m.oblock);

        if !self.has_unknown_flags(&m) {
            return Err(anyhow!("unknown flags in mapping"));
        }

        Ok(())
    }
}

//------------------------------------------

struct CheckHintVisitor<Width> {
    _not_used: PhantomData<Width>,
}

impl<Width> CheckHintVisitor<Width> {
    fn new() -> CheckHintVisitor<Width> {
        CheckHintVisitor {
            _not_used: PhantomData,
        }
    }
}

impl<Width: typenum::Unsigned> ArrayBlockVisitor<Hint<Width>> for CheckHintVisitor<Width> {
    fn visit(&self, _index: u64, _hint: Hint<Width>) -> anyhow::Result<()> {
        // TODO: check hints
        Ok(())
    }
}

//------------------------------------------

// TODO: ignore_non_fatal, clear_needs_check, auto_repair
pub struct CacheCheckOptions<'a> {
    pub dev: &'a Path,
    pub async_io: bool,
    pub sb_only: bool,
    pub skip_mappings: bool,
    pub skip_hints: bool,
    pub skip_discards: bool,
}

// TODO: thread pool, report
struct Context {
    engine: Arc<dyn IoEngine + Send + Sync>,
}

fn mk_context(opts: &CacheCheckOptions) -> anyhow::Result<Context> {
    let engine: Arc<dyn IoEngine + Send + Sync>;

    if opts.async_io {
        engine = Arc::new(AsyncIoEngine::new(
            opts.dev,
            MAX_CONCURRENT_IO,
            false,
        )?);
    } else {
        let nr_threads = std::cmp::max(8, num_cpus::get() * 2);
        engine = Arc::new(SyncIoEngine::new(opts.dev, nr_threads, false)?);
    }

    Ok(Context {
        engine,
    })
}

pub fn check(opts: CacheCheckOptions) -> anyhow::Result<()> {
    let ctx = mk_context(&opts)?;

    let engine = &ctx.engine;

    let sb = read_superblock(engine.as_ref(), SUPERBLOCK_LOCATION)?;

    if opts.sb_only {
        return Ok(());
    }

    // TODO: factor out into check_mappings()
    if !opts.skip_mappings {
        let w = ArrayWalker::new(engine.clone(), false);
        let c = Box::new(CheckMappingVisitor::new(sb.version));
        w.walk(c, sb.mapping_root)?;

        if sb.version >= 2 {
           // TODO: check dirty bitset
        }
    }

    if !opts.skip_hints && sb.hint_root != 0 {
        let w = ArrayWalker::new(engine.clone(), false);
        type Width = typenum::U4; // FIXME: check sb.policy_hint_size
        let c = Box::new(CheckHintVisitor::<Width>::new());
        w.walk(c, sb.hint_root)?;
    }

    if !opts.skip_discards {
        // TODO: check discard bitset
    }

    Ok(())
}

//------------------------------------------
