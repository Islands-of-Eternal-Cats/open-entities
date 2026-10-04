//! The render boundary: the per-tick position frame and the metadata delta beside it.

use open_entities::components::{MoveTarget, Position};
use open_entities::{
    Api, EntityComponents, EntityId, EntityMeta, FRAME_HEADER_LEN, FRAME_STRIDE, MetaDelta,
};

const TEMPLATES: &str = "entities:
  mover:
    faction: 1
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 5.0
  truck:
    faction: 1
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 5.0
    boardable: 2
  rock: {}
";

fn api() -> Api {
    let mut api = Api::new();
    api.load_templates_yaml(TEMPLATES).expect("templates load");
    api
}

fn spawn(api: &mut Api, template: &str, x: i32, y: i32) -> EntityId {
    api.spawn_entity(
        template,
        EntityComponents {
            position: Some(Position { x, y }),
            ..Default::default()
        },
    )
    .expect("spawn")
}

fn frame(api: &mut Api) -> Vec<i32> {
    let mut out = Vec::new();
    api.write_frame(&mut out);
    out
}

/// `[index, generation, x, y]` rows of a frame.
fn rows(frame: &[i32]) -> Vec<[i32; 4]> {
    assert_eq!(FRAME_STRIDE, 4);
    let (rows, rest) = frame[FRAME_HEADER_LEN..].as_chunks::<4>();
    assert!(rest.is_empty(), "a frame is whole rows");
    rows.to_vec()
}

fn as_i32(value: u32) -> i32 {
    i32::from_ne_bytes(value.to_ne_bytes())
}

fn changed_ids(delta: &MetaDelta) -> Vec<EntityId> {
    delta.changed.iter().map(|meta| meta.id).collect()
}

fn meta_of(delta: &MetaDelta, id: EntityId) -> &EntityMeta {
    delta
        .changed
        .iter()
        .find(|meta| meta.id == id)
        .expect("entity in the delta")
}

// --- frame -----------------------------------------------------------------------------------

#[test]
fn an_empty_world_writes_only_the_header() {
    let mut api = api();
    assert_eq!(frame(&mut api), vec![0, 0, 0]);
}

#[test]
fn the_frame_has_tick_count_and_one_row_per_positioned_entity_sorted_by_index() {
    let mut api = api();
    let a = spawn(&mut api, "mover", 1000, 2000);
    let b = spawn(&mut api, "rock", -3000, 4000);
    // A group has no position and is not drawn.
    api.create_group(1);
    api.step();
    api.step();

    let out = frame(&mut api);
    assert_eq!(&out[..FRAME_HEADER_LEN], &[2, 0, 2]);
    assert_eq!(out.len(), FRAME_HEADER_LEN + 2 * FRAME_STRIDE);
    assert_eq!(
        rows(&out),
        vec![
            [as_i32(a.index), as_i32(a.generation), 1000, 2000],
            [as_i32(b.index), as_i32(b.generation), -3000, 4000],
        ]
    );
}

#[test]
fn rows_stay_in_index_order_whatever_the_storage_order() {
    let mut api = api();
    let ids: Vec<EntityId> = (0..6)
        .map(|i| spawn(&mut api, "mover", i * 1000, 0))
        .collect();
    // Moving some of them into a different archetype reorders query iteration.
    api.order_move_to(&[ids[4], ids[1]], MoveTarget { x: 50_000, y: 0 });
    let indices: Vec<i32> = rows(&frame(&mut api)).iter().map(|row| row[0]).collect();
    let mut sorted = indices.clone();
    sorted.sort_unstable();
    assert_eq!(indices, sorted);
    assert_eq!(indices.len(), 6);
}

#[test]
fn the_tick_is_split_into_low_and_high_words() {
    let mut out = Vec::new();
    open_entities::export::write_frame_header(&mut out, 0x0000_0001_8000_0002, 7);
    assert_eq!(out, vec![as_i32(0x8000_0002), 1, 7]);
}

#[test]
fn a_frame_reuses_the_buffer_it_is_given() {
    let mut api = api();
    spawn(&mut api, "mover", 0, 0);
    let mut out = vec![9; 100];
    api.write_frame(&mut out);
    assert_eq!(out.len(), FRAME_HEADER_LEN + FRAME_STRIDE);
}

#[test]
fn a_despawned_entity_leaves_the_frame() {
    let mut api = api();
    let a = spawn(&mut api, "mover", 0, 0);
    let b = spawn(&mut api, "mover", 1000, 0);
    api.despawn(&[a]);
    let indices: Vec<i32> = rows(&frame(&mut api)).iter().map(|row| row[0]).collect();
    assert_eq!(indices, vec![as_i32(b.index)]);
}

// --- metadata delta --------------------------------------------------------------------------

#[test]
fn the_first_delta_carries_every_positioned_entity() {
    let mut api = api();
    let mover = spawn(&mut api, "mover", 0, 0);
    let truck = spawn(&mut api, "truck", 1000, 0);
    api.create_group(1);

    let delta = api.meta_delta();
    assert_eq!(changed_ids(&delta), vec![mover, truck]);
    assert!(delta.removed.is_empty());

    let meta = meta_of(&delta, truck);
    assert_eq!(meta.entity_type.as_deref(), Some("truck"));
    assert_eq!(meta.faction, Some(1));
    assert_eq!(meta.seats, Some(2));
    assert!(meta.mobile);
    assert_eq!(meta.aboard, None);
}

#[test]
fn nothing_changed_means_an_empty_delta_even_while_units_move() {
    let mut api = api();
    let mover = spawn(&mut api, "mover", 0, 0);
    api.order_move_to(&[mover], MoveTarget { x: 100_000, y: 0 });
    api.meta_delta();

    for _ in 0..5 {
        api.step();
    }
    let delta = api.meta_delta();
    assert!(delta.is_empty(), "positions are not metadata: {delta:?}");
}

#[test]
fn a_new_order_and_an_arrival_show_up_as_a_changed_move_target() {
    let mut api = api();
    let mover = spawn(&mut api, "mover", 0, 0);
    api.meta_delta();

    api.submit(open_entities::Command::MoveTo {
        ids: vec![mover],
        target: MoveTarget { x: 500, y: 0 },
    });
    api.step();
    let ordered = api.meta_delta();
    assert_eq!(
        meta_of(&ordered, mover).move_target,
        Some(MoveTarget { x: 500, y: 0 })
    );

    // 250 milli-units per tick: arrives on the second step.
    api.step();
    api.step();
    let arrived = api.meta_delta();
    assert_eq!(changed_ids(&arrived), vec![mover]);
    assert_eq!(meta_of(&arrived, mover).move_target, None);
}

#[test]
fn changes_over_several_steps_collect_into_one_delta() {
    let mut api = api();
    let a = spawn(&mut api, "mover", 0, 0);
    let b = spawn(&mut api, "mover", 0, 0);
    api.meta_delta();

    api.order_stop(&[a]);
    api.order_move_to(&[a], MoveTarget { x: 90_000, y: 0 });
    api.step();
    api.despawn(&[b]);
    api.step();
    api.step();

    let delta = api.meta_delta();
    assert_eq!(changed_ids(&delta), vec![a]);
    assert_eq!(delta.removed, vec![b]);
}

#[test]
fn boarding_and_groups_are_reported_as_ids() {
    let mut api = api();
    let mover = spawn(&mut api, "mover", 0, 0);
    let walker = spawn(&mut api, "mover", 50_000, 0);
    let truck = spawn(&mut api, "truck", 0, 0);
    let group = api.create_group(1);
    api.meta_delta();

    api.board(mover, truck).expect("boards");
    api.order_board(&[walker], truck).expect("walks to board");
    api.add_to_group(group, walker).expect("joins");
    let delta = api.meta_delta();

    assert_eq!(meta_of(&delta, mover).aboard, Some(truck));
    assert_eq!(meta_of(&delta, walker).boarding, Some(truck));
    assert_eq!(meta_of(&delta, walker).group, Some(group));
}

#[test]
fn mission_steering_rewriting_the_same_target_is_not_a_change() {
    let mut api = api();
    let a = spawn(&mut api, "mover", 0, 0);
    let group = api.create_group(1);
    api.add_to_group(group, a).expect("joins");
    let mission = api.create_mission(MoveTarget { x: 900_000, y: 0 }, 1000);
    api.assign_group(mission, group).expect("assigned");
    api.step();
    api.meta_delta();

    // The steering system inserts the same slot target every tick.
    api.step();
    api.step();
    assert!(api.meta_delta().is_empty());
}

#[test]
fn an_entity_spawned_and_despawned_between_two_calls_is_not_reported() {
    let mut api = api();
    api.meta_delta();
    let ghost = spawn(&mut api, "mover", 0, 0);
    api.step();
    api.despawn(&[ghost]);
    api.step();
    assert!(api.meta_delta().is_empty());
}

#[test]
fn the_delta_serializes_with_ids_and_map_units() {
    let mut api = api();
    let mover = spawn(&mut api, "mover", 0, 0);
    api.order_move_to(&[mover], MoveTarget { x: 1500, y: 0 });
    let value = serde_json::to_value(api.meta_delta()).expect("serialize");
    let row = &value["changed"][0];
    assert_eq!(row["id"]["index"], mover.index);
    assert_eq!(row["entity_type"], "mover");
    assert_eq!(row["move_target"]["x"], 1.5);
    assert_eq!(row["mobile"], true);
    assert!(row.get("aboard").is_none(), "absent fields are omitted");
    assert_eq!(value["removed"], serde_json::json!([]));
}

#[test]
fn reading_the_boundary_does_not_change_the_state_hash() {
    let mut with_reads = api();
    let mut without = api();
    for api in [&mut with_reads, &mut without] {
        let a = spawn(api, "mover", 0, 0);
        api.order_move_to(&[a], MoveTarget { x: 9000, y: 3000 });
    }
    for _ in 0..40 {
        with_reads.step();
        frame(&mut with_reads);
        with_reads.meta_delta();
        without.step();
    }
    assert_eq!(with_reads.state_hash(), without.state_hash());
}
