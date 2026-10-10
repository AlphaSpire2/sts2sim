use sts2_engine::{
    ChoiceCoverage, ChoiceDestination, ChoiceIdentity, ChoiceOperation, ChoiceOperationKind,
    ChoiceScope, ChoiceSemantics, ChoiceSubject,
};

#[test]
fn unknown_partial_and_known_round_trip_without_inventing_an_effect() {
    for coverage in [
        ChoiceCoverage::Unknown,
        ChoiceCoverage::Partial,
        ChoiceCoverage::Known,
    ] {
        let mut semantics = ChoiceSemantics::deck_removal();
        semantics.coverage = coverage;
        if coverage == ChoiceCoverage::Unknown {
            semantics.operations.clear();
        }
        semantics.validate().unwrap();
        let bytes = serde_json::to_vec(&semantics).unwrap();
        assert_eq!(semantics, serde_json::from_slice(&bytes).unwrap());
    }
    assert!(ChoiceSemantics::known(Vec::new()).validate().is_err());
}

#[test]
fn compound_effect_preserves_order_subject_and_scope() {
    let transform = ChoiceOperation::new(ChoiceOperationKind::Transform, ChoiceScope::Run).rolled();
    let upgrade = ChoiceOperation::new(ChoiceOperationKind::Upgrade, ChoiceScope::Run)
        .on(ChoiceSubject::Replacement);
    let original = ChoiceSemantics::known(vec![transform.clone(), upgrade.clone()])
        .from_origin(ChoiceIdentity::DeckCard);
    original.validate().unwrap();
    let reversed =
        ChoiceSemantics::known(vec![upgrade, transform]).from_origin(ChoiceIdentity::DeckCard);
    assert_ne!(
        serde_json::to_vec(&original).unwrap(),
        serde_json::to_vec(&reversed).unwrap()
    );
    assert_eq!(
        ChoiceSemantics::deck_removal().operations[0].destination,
        ChoiceDestination::Removed
    );
    assert_ne!(ChoiceDestination::Unknown, ChoiceDestination::NotApplicable);
}
