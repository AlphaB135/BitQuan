use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bitquan_consensus::{validate_block, validate_transaction};
use bitquan_types::{Block, Transaction, TxContext, NetworkId};

fn bench_validate_transaction(c: &mut Criterion) {
    let tx = Transaction::default();
    let ctx = TxContext {
        network_id: NetworkId::Mainnet,
        genesis_hash: [0u8; 32],
    };

    c.bench_function("validate_transaction", |b| {
        b.iter(|| {
            validate_transaction(black_box(&tx), black_box(&ctx))
        });
    });
}

fn bench_validate_block(c: &mut Criterion) {
    let block = Block::default();
    let height = 1000u64;

    c.bench_function("validate_block", |b| {
        b.iter(|| {
            validate_block(black_box(&block), black_box(height))
        });
    });
}

fn bench_calculate_block_weight(c: &mut Criterion) {
    let block = Block::default();

    c.bench_function("calculate_block_weight", |b| {
        b.iter(|| {
            black_box(&block).calculate_weight()
        });
    });
}

criterion_group!(benches, bench_validate_transaction, bench_validate_block, bench_calculate_block_weight);
criterion_main!(benches);
