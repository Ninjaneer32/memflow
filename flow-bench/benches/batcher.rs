use criterion::*;

use flow_core::mem::{PhysicalMemory, PhysicalReadIterator, PhysicalWriteIterator};
use flow_core::Result;
use flow_core::{Address, Length};

//use flow_core::mem::dummy::DummyMemory as Memory;

struct NullMem {}

impl NullMem {
    pub fn new(_: Length) -> Self {
        Self {}
    }
}

impl PhysicalMemory for NullMem {
    fn phys_read_iter<'a, PI: PhysicalReadIterator<'a>>(&'a mut self, iter: PI) -> Result<()> {
        black_box(iter.count());
        Ok(())
    }

    fn phys_write_iter<'a, PI: PhysicalWriteIterator<'a>>(&'a mut self, iter: PI) -> Result<()> {
        black_box(iter.count());
        Ok(())
    }
}

use NullMem as Memory;

use rand::prelude::*;
use rand::{prng::XorShiftRng as CurRng, Rng, SeedableRng};

static mut TSLICE: [[u8; 16]; 0x10000] = [[0; 16]; 0x10000];

fn read_test_nobatcher<T: PhysicalMemory>(
    chunk_size: usize,
    mem: &mut T,
    mut rng: CurRng,
    size: Length,
) {
    let base_addr = Address::from(rng.gen_range(0, size.as_u64()));

    let _ = black_box(
        mem.phys_read_iter(
            unsafe { TSLICE }
                .iter_mut()
                .map(|buf| {
                    (
                        (base_addr + Length::from(rng.gen_range(0, 0x2000))).into(),
                        &mut buf[..],
                    )
                })
                .take(chunk_size),
        ),
    );
}

fn read_test_batcher<T: PhysicalMemory>(
    chunk_size: usize,
    mem: &mut T,
    mut rng: CurRng,
    size: Length,
) {
    let base_addr = Address::from(rng.gen_range(0, size.as_u64()));

    let mut batcher = mem.get_batcher();
    batcher.read_prealloc(chunk_size);

    for i in unsafe { TSLICE.iter_mut().take(chunk_size) } {
        batcher.read_into(
            (base_addr + Length::from(rng.gen_range(0, 0x2000))).into(),
            i,
        );
    }

    let _ = black_box(batcher.commit_rw());
}

fn read_test_with_ctx<T: PhysicalMemory>(
    bench: &mut Bencher,
    chunk_size: usize,
    use_batcher: bool,
    mem: &mut T,
) {
    let rng = CurRng::from_rng(thread_rng()).unwrap();

    let mem_size = Length::from_mb(64);

    if !use_batcher {
        bench.iter(|| read_test_nobatcher(chunk_size, mem, rng.clone(), mem_size));
    } else {
        bench.iter(|| read_test_batcher(chunk_size, mem, rng.clone(), mem_size));
    }
}

fn chunk_read_params<T: PhysicalMemory>(
    group: &mut BenchmarkGroup<'_, measurement::WallTime>,
    func_name: String,
    use_batcher: bool,
    initialize_ctx: &dyn Fn() -> T,
) {
    for &chunk_size in [1, 4, 16, 64, 256, 1024, 4096, 16384, 65536].iter() {
        group.throughput(Throughput::Bytes(chunk_size));
        group.bench_with_input(
            BenchmarkId::new(func_name.clone(), chunk_size),
            &chunk_size,
            |b, &chunk_size| {
                read_test_with_ctx(
                    b,
                    black_box(chunk_size as usize),
                    use_batcher,
                    &mut initialize_ctx(),
                )
            },
        );
    }
}

fn chunk_read<T: PhysicalMemory>(
    c: &mut Criterion,
    backend_name: &str,
    initialize_ctx: &dyn Fn() -> T,
) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    let group_name = format!("{}_batched_read", backend_name);

    let mut group = c.benchmark_group(group_name.clone());
    group.plot_config(plot_config);

    chunk_read_params(
        &mut group,
        format!("{}_without", group_name),
        false,
        initialize_ctx,
    );
    chunk_read_params(
        &mut group,
        format!("{}_with", group_name),
        true,
        initialize_ctx,
    );
}
criterion_group! {
    name = dummy_read;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(300))
        .measurement_time(std::time::Duration::from_millis(2700));
    targets = dummy_read_group
}

fn dummy_read_group(c: &mut Criterion) {
    chunk_read(c, "dummy", &|| Memory::new(Length::from_mb(64)));
}

criterion_main!(dummy_read);