use crate::gk;

#[test]
fn insertion() {
    let sum = gk::Summary::new();
    sum.insert(1.);
}