//! Gameplay ends after the last required boss and its combat effects settle.

use serde_json::json;
use sts2_engine::{
    Action, DecisionContext, MapCoord, MapPoint, MapPointType, RunResult, Simulator,
};
use sts2_replay::{AdapterErrorCode, Record, ReplayAdapter};

fn fixture(double_boss: bool, second: bool) -> Simulator {
    let mut state = sts2_content::final_boss_fixture().unwrap().state().clone();
    let run = state.run.as_mut().unwrap();
    if double_boss {
        let map = &mut run.maps[run.current_act];
        let coord = MapCoord {
            col: map.boss.col,
            row: map.boss.row + 1,
        };
        map.second_boss = Some(coord);
        map.points
            .iter_mut()
            .find(|p| p.coord == map.boss)
            .unwrap()
            .children = vec![coord];
        map.points.push(MapPoint {
            coord,
            point_type: MapPointType::Boss,
            children: vec![],
            can_modify: true,
        });
        if second {
            run.current_coord = Some(coord);
        }
    }
    state.run_player.current_hp = 30;
    let combat = state.combat.as_mut().unwrap();
    let player = combat.player.creature_id;
    combat
        .creatures
        .iter_mut()
        .find(|c| c.id == player)
        .unwrap()
        .current_hp = 30;
    Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap()
}

fn win(sim: &mut Simulator) {
    let action = sim
        .legal_actions()
        .iter()
        .find(|a| matches!(a, Action::PlayCard { .. }))
        .unwrap()
        .clone();
    sim.step(action).unwrap();
}

#[test]
fn last_required_boss_ends_gameplay_without_a_departure() {
    for double_boss in [false, true] {
        let mut sim = fixture(double_boss, true);
        let before = sim.state().run.as_ref().unwrap().clone();
        win(&mut sim);
        assert_eq!(sim.state().terminal, Some(RunResult::Victory));
        assert!(matches!(
            sim.decision(),
            DecisionContext::Terminal {
                result: RunResult::Victory
            }
        ));
        assert_eq!(sim.legal_actions(), []);
        let run = sim.state().run.as_ref().unwrap();
        assert_eq!(run.current_room_model, before.current_room_model);
        assert_eq!(run.current_coord, before.current_coord);
        assert_eq!(run.floor, before.floor);
        assert_eq!(
            run.acts[run.current_act].events_visited,
            before.acts[before.current_act].events_visited
        );
        assert_eq!(
            sim.state().run_player.current_hp,
            36,
            "Burning Blood settles before victory"
        );
        assert!(sim.state().combat.as_ref().unwrap().has_ended);
    }
}

#[test]
fn first_of_two_bosses_does_not_end_gameplay() {
    let mut sim = fixture(true, false);
    win(&mut sim);
    assert!(sim.state().terminal.is_none());
    assert_ne!(sim.legal_actions(), []);
    assert!(
        !sim.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::AdvanceAct))
    );
}

#[test]
fn earlier_act_boss_does_not_end_gameplay() {
    let mut state = fixture(false, false).state().clone();
    let run = state.run.as_mut().unwrap();
    run.current_act = 0;
    run.current_coord = Some(run.maps[0].boss);
    let mut sim = Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap();
    win(&mut sim);
    assert!(sim.state().terminal.is_none());
    assert!(
        sim.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::AdvanceAct))
    );
}

fn won_replay() -> ReplayAdapter {
    let mut adapter = ReplayAdapter::new(fixture(false, false)).verifying_observations();
    adapter.accept(&Record { sequence: 1, kind: "combat.play_card".into(), payload: json!({"data": {
        "combat_card_index": 0, "card_model": "CARD.STRIKE_IRONCLAD",
        "card": {"model_id":"CARD.STRIKE_IRONCLAD", "upgrade_level":0, "enchantment":null, "properties":null, "floor_added_to_deck":null},
        "target": {"combat_id": 1, "model_id": "MONSTER.QUEEN"}, "target_id": 1
    }}) }).unwrap();
    assert_eq!(
        adapter.simulator().state().terminal,
        Some(RunResult::Victory)
    );
    adapter
}

fn observation(adapter: &ReplayAdapter) -> Record {
    Record {
        sequence: 2,
        kind: "state.observe".into(),
        payload: json!({"after":1,
        "context":"action.settled", "state":sts2_replay::observation::render(adapter.simulator())}),
    }
}

fn departure() -> Record {
    Record {
        sequence: 3,
        kind: "act.next".into(),
        payload: json!({"data": {}}),
    }
}

#[test]
fn legacy_presentation_is_excluded_only_after_verifying_final_combat() {
    let mut adapter = won_replay();
    adapter.accept(&observation(&adapter)).unwrap();
    adapter.accept(&departure()).unwrap();
    let before = adapter.simulator().snapshot();
    adapter
        .accept(&Record {
            sequence: 4,
            kind: "state.observe".into(),
            payload: json!({
        "after":3, "context":"room.enter", "state":{"arbitrary_presentation":true}}),
        })
        .unwrap();
    adapter.verify_pending_observation().unwrap();
    assert_eq!(adapter.simulator().snapshot(), before);
    assert_eq!(adapter.compared_observations(), 1);
    assert_eq!(adapter.post_victory_records(), 2);
}

#[test]
fn final_combat_mismatch_is_not_hidden_by_the_presentation_boundary() {
    let mut adapter = won_replay();
    let mut record = observation(&adapter);
    record.payload["state"]["players"][0]["current_hp"] = json!(999);
    adapter.accept(&record).unwrap();
    assert_eq!(
        adapter.accept(&departure()).unwrap_err().code,
        AdapterErrorCode::StateDivergence
    );
    assert_eq!(adapter.post_victory_records(), 0);
}

#[test]
fn new_recording_final_observation_is_still_verified() {
    let mut adapter = won_replay();
    adapter
        .accept(&Record {
            sequence: 2,
            kind: "run.finish".into(),
            payload: json!({"data":{"result":"VICTORY"}}),
        })
        .unwrap();
    let mut record = observation(&adapter);
    record.sequence = 3;
    record.payload["after"] = json!(2);
    record.payload["context"] = json!("run.end");
    record.payload["state"]["players"][0]["current_hp"] = json!(999);
    adapter.accept(&record).unwrap();
    assert_eq!(
        adapter.verify_pending_observation().unwrap_err().code,
        AdapterErrorCode::StateDivergence
    );
}
