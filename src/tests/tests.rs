use crate::mean::MeanOperation;

use crate::Operation;
#[test]
fn single_entity() {
    let mut state = MeanOperation::create();
    for n in vec![3., 14., 15.] {
        state.update(n);
    }
    assert_eq!(state.finish(), (3. + 14. + 15.) / 3.);
}

#[test]
fn empty_state() {
    assert!(MeanOperation::create().finish().is_nan());
}

#[test]
fn merge_entities() {
    let mut state = MeanOperation::create();
    for n in vec![3., 14., 15.] {
        state.update(n);
    }
    let mut state2 = MeanOperation::create();
    for n in vec![92., 65., 35.] {
        state2.update(n);
    }
    state.merge_with(state2);
    assert_eq!(state.finish(), (3. + 14. + 15. + 92. + 65. + 35.) / 6.);
}
