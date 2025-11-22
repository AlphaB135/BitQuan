use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bitquan_mempool::Mempool;
use bitquan_types::Transaction;

fn bench_add_transaction(c: &mut Criterion) {
    let mempool = Mempool::new(300_000_000);
    let tx = Transaction::default();

    c.bench_function("mempool_add_transaction", |b| {
        b.iter(|| {
            let _ = mempool.add_transaction(black_box(tx.clone()));
        });
    });
}

fn bench_get_transactions(c: &mut Criterion) {
    let mempool = Mempool::new(300_000_000);

    // Add some transactions first
    for _ in 0..100 {
        let _ = mempool.add_transaction(Transaction::default());
    }

    c.bench_function("mempool_get_transactions", |b| {
        b.iter(|| {
            mempool.get_transactions(black_box(100))
        });
    });
}

fn bench_remove_transactions(c: &mut Criterion) {
    let mempool = Mempool::new(300_000_000);
    let tx = Transaction::default();
    let txid = tx.txid();
    let _ = mempool.add_transaction(tx);

    c.bench_function("mempool_remove_transaction", |b| {
        b.iter(|| {
            mempool.remove_transaction(black_box(&txid))
        });
    });
}

criterion_group!(benches, bench_add_transaction, bench_get_transactions, bench_remove_transactions);
criterion_main!(benches);
