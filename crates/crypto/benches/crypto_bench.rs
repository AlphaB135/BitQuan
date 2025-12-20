use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pqc_dilithium_seeded::{verify, Keypair};

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("generate_dilithium_keypair", |b| {
        b.iter(|| Keypair::generate());
    });
}

fn bench_sign_message(c: &mut Criterion) {
    let keypair = Keypair::generate();
    let message = b"Hello, BitQuan!";

    c.bench_function("sign_message", |b| {
        b.iter(|| keypair.sign(black_box(message)));
    });
}

fn bench_verify_signature(c: &mut Criterion) {
    let keypair = Keypair::generate();
    let message = b"Hello, BitQuan!";
    let signature = keypair.sign(message);

    c.bench_function("verify_signature", |b| {
        b.iter(|| verify(black_box(&signature), black_box(message), &keypair.public));
    });
}

criterion_group!(
    benches,
    bench_keypair_generation,
    bench_sign_message,
    bench_verify_signature
);
criterion_main!(benches);
