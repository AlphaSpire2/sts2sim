//! Registry-wide coverage includes upgraded bodies, hooks, and event pages.
use std::collections::BTreeMap;

use serde_json::Value;
use sts2_core::ModelId;
use sts2_engine::{CardSelectCmd, ChoiceCoverage};

fn visit(value: &Value, owner: &str, found: &mut BTreeMap<String, usize>) {
    match value {
        Value::Object(object) => {
            if let Some(selection) = object.get("CardSelect") {
                let command: CardSelectCmd = serde_json::from_value(selection.clone()).unwrap();
                let semantics = command.semantics();
                assert_eq!(
                    semantics.coverage,
                    ChoiceCoverage::Known,
                    "{owner}: {command:?}"
                );
                semantics
                    .validate()
                    .unwrap_or_else(|error| panic!("{owner}: {error}"));
                assert!(
                    semantics.operations.len() <= 3,
                    "inventory bound changed: {owner}"
                );
                *found.entry(owner.to_owned()).or_default() += 1;
            }
            for child in object.values() {
                visit(child, owner, found);
            }
        }
        Value::Array(array) => {
            for child in array {
                visit(child, owner, found);
            }
        }
        _ => {}
    }
}

#[test]
fn every_registered_selection_has_complete_public_semantics() {
    let mut found = BTreeMap::new();
    for character in ["IRONCLAD", "SILENT", "DEFECT", "NECROBINDER", "REGENT"] {
        for ascension in [0, 10] {
            let registry = sts2_content::registry_at(
                &ModelId::new("CHARACTER", character).unwrap(),
                ascension,
            );
            for id in registry.registered_model_ids() {
                let value = match id.category() {
                    "CARD" => registry.card(&id).map(|v| serde_json::to_value(v).unwrap()),
                    "POWER" => registry
                        .power(&id)
                        .map(|v| serde_json::to_value(v).unwrap()),
                    "RELIC" => registry
                        .relic(&id)
                        .map(|v| serde_json::to_value(v).unwrap()),
                    "POTION" => registry
                        .potion(&id)
                        .map(|v| serde_json::to_value(v).unwrap()),
                    "MONSTER" => registry
                        .monster(&id)
                        .map(|v| serde_json::to_value(v).unwrap()),
                    "EVENT" => registry
                        .event(&id)
                        .map(|v| serde_json::to_value(v).unwrap()),
                    _ => None,
                };
                if let Some(value) = value {
                    visit(&value, &id.to_string(), &mut found);
                }
            }
        }
    }
    for id in [
        "CARD.ARMAMENTS",
        "CARD.TRUE_GRIT",
        "CARD.BURNING_PACT",
        "POWER.ENTROPY_POWER",
        "RELIC.ASTROLABE",
        "CARD.HAND_TRICK",
        "CARD.TRANSFIGURE",
        "CARD.NIGHTMARE",
        "RELIC.PAELS_TOOTH",
    ] {
        assert!(found.contains_key(id), "missing core producer {id}");
    }
    assert!(
        found.len() > 78,
        "helpers must expand into individual producers"
    );
}

fn selections(value: &Value, result: &mut Vec<CardSelectCmd>) {
    match value {
        Value::Object(object) => {
            if let Some(selection) = object.get("CardSelect") {
                result.push(serde_json::from_value(selection.clone()).unwrap());
            }
            for child in object.values() {
                selections(child, result);
            }
        }
        Value::Array(array) => {
            for child in array {
                selections(child, result);
            }
        }
        _ => {}
    }
}

#[test]
fn compound_producers_preserve_parameters_subjects_and_public_conditions() {
    use sts2_engine::{
        ChoiceCondition as C, ChoiceDuration as D, ChoiceOperationKind as K, ChoiceSubject as S,
        ChoiceTiming as T,
    };
    let registry = sts2_content::standard_registry();
    for (name, kinds) in [
        ("CARD.TRANSFIGURE", vec![K::AddCost, K::AddReplay]),
        ("CARD.NIGHTMARE", vec![K::Copy]),
        ("RELIC.ASTROLABE", vec![K::Transform, K::Upgrade]),
        ("RELIC.PAELS_TOOTH", vec![K::Store, K::Upgrade, K::Move]),
        ("RELIC.PAELS_GROWTH", vec![K::Enchant]),
        ("POTION.LIQUID_MEMORIES", vec![K::SetCost, K::Move]),
    ] {
        let id: ModelId = name.parse().unwrap();
        let body = match id.category() {
            "CARD" => serde_json::to_value(registry.card(&id).unwrap()).unwrap(),
            "RELIC" => serde_json::to_value(registry.relic(&id).unwrap()).unwrap(),
            _ => serde_json::to_value(registry.potion(&id).unwrap()).unwrap(),
        };
        let mut found = Vec::new();
        selections(&body, &mut found);
        assert!(!found.is_empty(), "{name}");
        for command in found {
            let ops = &command.semantics().operations;
            assert_eq!(
                ops.iter().map(|op| op.kind).collect::<Vec<_>>(),
                kinds,
                "{name}"
            );
            match name {
                "CARD.TRANSFIGURE" => {
                    assert_eq!(ops[0].condition, C::WritableEnergyCost);
                    assert_eq!(ops[1].condition, C::Always);
                    assert_eq!((ops[0].amount, ops[1].amount), (1, 1));
                }
                "CARD.NIGHTMARE" => {
                    assert_eq!(ops[0].count, 3);
                    assert_eq!(ops[0].timing, T::NextTurn);
                }
                "RELIC.ASTROLABE" => {
                    assert!(ops[0].rolled);
                    assert_eq!(ops[1].subject, S::Replacement);
                }
                "RELIC.PAELS_TOOTH" => {
                    assert_eq!(ops[1].timing, T::AfterEachCombat);
                    assert_eq!(ops[2].timing, T::AfterEachCombat);
                }
                "RELIC.PAELS_GROWTH" => assert_eq!(ops[0].amount, 4),
                "POTION.LIQUID_MEMORIES" => {
                    assert_eq!(ops[0].amount, 0);
                    assert_eq!(ops[0].duration, D::UntilPlayedOrTurnEnds);
                }
                _ => unreachable!(),
            }
        }
    }
}
