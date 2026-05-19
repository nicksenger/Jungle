use super::{grid_note_to_note, GridNote, Kind, Position};
use crate::instrumentation::{LeadGuitarArticulation, Note};

const INTRO: &[GridNote] = &[];

const VERSE1: &[GridNote] = &[
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 49.756098,
        position: Position {
            bar: 26,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 50.243902,
        position: Position {
            bar: 26,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 50.731707,
        position: Position {
            bar: 27,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 51.219512,
        position: Position {
            bar: 27,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 51.707317,
        position: Position {
            bar: 27,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 52.195122,
        position: Position {
            bar: 27,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 52.682927,
        position: Position {
            bar: 28,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 53.170732,
        position: Position {
            bar: 28,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
];

const CHORUS1: &[GridNote] = &[];

const VERSE2: &[GridNote] = &[
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 77.073171,
        position: Position {
            bar: 40,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 77.560976,
        position: Position {
            bar: 40,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 78.04878,
        position: Position {
            bar: 41,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 78.536585,
        position: Position {
            bar: 41,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 79.02439,
        position: Position {
            bar: 41,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 79.512195,
        position: Position {
            bar: 41,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 80.0,
        position: Position {
            bar: 42,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 80.487805,
        position: Position {
            bar: 42,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
];

const CHORUS2: &[GridNote] = &[];

const SOLO1: &[GridNote] = &[];

const VERSE3: &[GridNote] = &[
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 120.0,
        position: Position {
            bar: 62,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 120.487805,
        position: Position {
            bar: 62,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 120.97561,
        position: Position {
            bar: 63,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 121.463415,
        position: Position {
            bar: 63,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 121.95122,
        position: Position {
            bar: 63,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 122.439024,
        position: Position {
            bar: 63,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 122.926829,
        position: Position {
            bar: 64,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 123.414634,
        position: Position {
            bar: 64,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
];

const CHORUS3: &[GridNote] = &[];

const BRIDGE1: &[GridNote] = &[];

const SOLO2: &[GridNote] = &[];

const BRIDGE2: &[GridNote] = &[
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 223.414634,
        position: Position {
            bar: 115,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 223.902439,
        position: Position {
            bar: 115,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 224.390244,
        position: Position {
            bar: 116,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 224.878049,
        position: Position {
            bar: 116,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 225.365854,
        position: Position {
            bar: 116,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 225.853659,
        position: Position {
            bar: 116,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 226.341463,
        position: Position {
            bar: 117,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 226.829268,
        position: Position {
            bar: 117,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
];

const FINALE: &[GridNote] = &[
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 231.219512,
        position: Position {
            bar: 119,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 231.707317,
        position: Position {
            bar: 119,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 232.195122,
        position: Position {
            bar: 120,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 232.682927,
        position: Position {
            bar: 120,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 233.170732,
        position: Position {
            bar: 120,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 233.658537,
        position: Position {
            bar: 120,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 234.146341,
        position: Position {
            bar: 121,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 234.634146,
        position: Position {
            bar: 121,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 239.02439,
        position: Position {
            bar: 123,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 239.512195,
        position: Position {
            bar: 123,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 240.0,
        position: Position {
            bar: 124,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 240.487805,
        position: Position {
            bar: 124,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 240.97561,
        position: Position {
            bar: 124,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 241.463415,
        position: Position {
            bar: 124,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 241.95122,
        position: Position {
            bar: 125,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 242.439024,
        position: Position {
            bar: 125,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 71,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 246.829268,
        position: Position {
            bar: 127,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 247.317073,
        position: Position {
            bar: 127,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 247.804878,
        position: Position {
            bar: 128,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 66,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 248.292683,
        position: Position {
            bar: 128,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 73,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 248.780488,
        position: Position {
            bar: 128,
            beat: 3,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 72,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 249.268293,
        position: Position {
            bar: 128,
            beat: 4,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 70,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 249.756098,
        position: Position {
            bar: 129,
            beat: 1,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
    GridNote {
        midi: 68,
        kind: Kind::Quarter,
        beats: 1.0,
        d_sec: 0.487805,
        t_sec: 250.243902,
        position: Position {
            bar: 129,
            beat: 2,
            beat_offset_num: 0,
            beat_offset_den: 1,
        },
    },
];

const SECTIONS: [&[GridNote]; 12] = [
    INTRO, VERSE1, CHORUS1, VERSE2, CHORUS2, SOLO1, VERSE3, CHORUS3, BRIDGE1, SOLO2, BRIDGE2,
    FINALE,
];

pub fn vocals_score(bpm: f32) -> Vec<Note<LeadGuitarArticulation>> {
    SECTIONS
        .into_iter()
        .flatten()
        .copied()
        .map(|note| grid_note_to_note(note, bpm))
        .collect()
}
