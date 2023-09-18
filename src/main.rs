use rust_arc::Arc;

fn main() {
    let a = Arc::new(1);
    let b = Arc::clone(&a);
    let c = b.clone();
}
