use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bitquan_crypto::{Keypair, sign, verify};
use bitquan_types::{Transaction, TxContext, NetworkId};

fn bench_sign_transaction(c: &mut Criterion) {
    let keypair = Keypair::generate();
    let mut tx = Transaction::default();
    let ctx = TxContext {
        network_id: NetworkId::Mainnet,
        genesis_hash: [0u8; 32],
    };
    
    c.bench_function("sign_transaction", |b| {
        b.iter(|| {
            sign(black_box(&keypair), black_box(&tx), black_box(&ctx))
        });
    });
}

fn bench_verify_signature(c: &mut Criterion) {
    let keypair = Keypair::generate();
    let tx = Transaction::default();
    let ctx = TxContext {
        network_id: NetworkId::Mainnet,
        genesis_hash: [0u8; 32],
    };
    let signature = sign(&keypair, &tx, &ctx).unwrap();
    
    c.bench_function("verify_signature", |b| {
        b.iter(|| {
            verify(black_box(&keypair.public()), black_box(&signature), black_box(&tx), black_box(&ctx))
        });
    });
}

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("generate_keypair", |b| {
        b.iter(|| {
            Keypair::generate()
        });
    });
}

criterion_group!(benches, bench_sign_transaction, bench_verify_signature, bench_keypair_generation);
criterion_main!(benches);
