use uuid::Uuid;

use crate::packets::TAState;

#[derive(juniper::GraphQLObject, Default)]
pub struct User {
    id: Uuid,
    name: String,
    user_id: String,
    play_state: PlayState,
    download_state: DownloadState,
    team: Option<Team>,
    mod_list: Vec<String>,
    stream_delay_ms: i32,
    stream_sync_start_ms: i32,
}

#[repr(i32)]
#[derive(juniper::GraphQLEnum, Default)]
pub enum PlayState {
    #[default]
    Waiting = 0,
    InGame = 1
}

#[repr(i32)]
#[derive(juniper::GraphQLEnum, Default)]
pub enum DownloadState {
    #[default]
    None = 0,
    Downloading = 1,
    Downloaded = 2,
    DownloadError = 3
}

#[derive(juniper::GraphQLObject, Default)]
pub struct Team {
    id: Uuid,
    name: String,
}

#[derive(juniper::GraphQLObject)]
pub struct Match {
    id: Uuid,
    players: Vec<User>,
    teams: Vec<Team>,
    coordinators: Vec<User>,
    current_map: Option<Map>,
    scores: Vec<Score>
}

#[derive(juniper::GraphQLObject)]
pub struct Map {
    id: String,
    name: String,
    difficulty: i32,
    // characteristic: String,
    modifiers: Vec<String>,
}

#[derive(juniper::GraphQLObject)]
pub struct Score {
    owner_id: Uuid,

    score: i32,
    score_with_modifiers: i32,
    max_score: i32,
    max_score_with_modifiers: i32,
    combo: i32,
    player_health: f64,
    accuracy: f64,
    song_position: f64,
    notes_missed: i32,
    bad_cuts: i32,
    bomb_hits: i32,
    wall_hits: i32,
    max_combo: i32,
    left_hand_hits: i32,
    left_hand_misses: i32,
    left_hand_bad_cut: i32,
    right_hand_hits: i32,
    right_hand_misses: i32,
    right_hand_bad_cut: i32,
}

#[derive(juniper::GraphQLObject)]
pub struct GQLTAState {
    pub coordinators: Vec<User>,
    pub players: Vec<User>,
    pub matches: Vec<Match>,
}

#[derive(juniper::GraphQLObject, Clone)]
pub struct Page {
    pub data: Vec<PageData>,
    pub path: String
}

#[derive(juniper::GraphQLObject, Clone)]
pub struct PageData {
    pub key: String,
    pub value: String
}

#[derive(juniper::GraphQLInputObject, Clone)]
pub struct InputPage {
    pub data: Vec<InputPageData>,
    pub path: String
}

#[derive(juniper::GraphQLInputObject, Clone)]
pub struct InputPageData {
    pub key: String,
    pub value: String
}

#[derive(juniper::GraphQLObject, Clone)]
pub struct GQLOverState {
    pub page: Page,
}

impl GQLOverState {
    pub fn new() -> Self {
        Self {
            page: Page {
                path: "/".to_string(),
                data: vec![]
            }
        }
    }
}

impl InputPage {
    pub fn into_page(self) -> Page {
        Page {
            path: self.path,
            data: self.data.into_iter().map(|f| PageData {
                key: f.key, value: f.value}
            ).collect()
        }
    }
}

impl TAState {
    pub async fn into_gql(&self) -> GQLTAState {
        GQLTAState {
            players: self
                .players
                .iter()
                .map(|p| User {
                    id: Uuid::parse_str(&p.guid).unwrap(),
                    user_id: p.user_id.clone(),
                    name: p.name.clone(),
                    play_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::PlayState>(&p.play_state) },
                    download_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::DownloadState>(&p.download_state) },
                    team: p.team.as_ref().map(|t| Team {
                        id: Uuid::parse_str(&t.id).unwrap(),
                        name: t.name.clone(),
                    }),
                    mod_list: p.mod_list.clone(),
                    stream_delay_ms: p.stream_delay_ms as i32,
                    stream_sync_start_ms: p.stream_sync_start_ms as i32,
                })
                .collect(),
            coordinators: self
                .coordinators
                .iter()
                .map(|p| User {
                    id: Uuid::parse_str(&p.guid).unwrap(),
                    name: p.name.clone(),
                    user_id: p.user_id.clone(),
                    play_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::PlayState>(&p.play_state) },
                    download_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::DownloadState>(&p.download_state) },
                    team: p.team.as_ref().map(|t| Team {
                        id: Uuid::parse_str(&t.id).unwrap(),
                        name: t.name.clone(),
                    }),
                    mod_list: p.mod_list.clone(),
                    stream_delay_ms: p.stream_delay_ms as i32,
                    stream_sync_start_ms: p.stream_sync_start_ms as i32,
                })
                .collect(),
            matches: self
                .matches
                .iter()
                .map(|m| Match {
                    id: Uuid::parse_str(&m.guid).unwrap(),
                    players: m
                        .associated_users
                        .iter()
                        .filter_map(|u| {
                            self.players
                                .iter()
                                .find(|p| p.guid == *u)
                                .map(|p| User {
                                    id: Uuid::parse_str(&p.guid).unwrap(),
                                    name: p.name.clone(),
                                    user_id: p.user_id.clone(),
                                    play_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::PlayState>(&p.play_state) },
                                    download_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::DownloadState>(&p.download_state) },
                                    team: p.team.as_ref().map(|t| Team {
                                        id: Uuid::parse_str(&t.id).unwrap(),
                                        name: t.name.clone(),
                                    }),
                                    mod_list: p.mod_list.clone(),
                                    stream_delay_ms: p.stream_delay_ms as i32,
                                    stream_sync_start_ms: p.stream_sync_start_ms as i32,
                                })
                        })
                        .collect(),
                    teams: m
                        .associated_users
                        .iter()
                        .filter_map(|u| {
                            self.players
                                .iter()
                                .find(|p| p.guid == *u)
                                .map(|p| p.team.as_ref().map(|t| Team {
                                    id: Uuid::parse_str(&t.id).unwrap(),
                                    name: t.name.clone(),
                                }))
                                .flatten()
                        }).collect(),
                    coordinators: m
                        .associated_users
                        .iter()
                        .filter_map(|u| {
                            self.coordinators
                                .iter()
                                .find(|p| p.guid == *u)
                                .map(|c| User {
                                    id: Uuid::parse_str(&c.guid).unwrap(),
                                    name: c.name.clone(),
                                    user_id: c.user_id.clone(),
                                    play_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::PlayState>(&c.play_state) },
                                    download_state: unsafe { std::mem::transmute_copy::<i32, crate::structs::DownloadState>(&c.download_state) },
                                    team: c.team.as_ref().map(|t| Team {
                                        id: Uuid::parse_str(&t.id).unwrap(),
                                        name: t.name.clone(),
                                    }),
                                    mod_list: c.mod_list.clone(),
                                    stream_delay_ms: c.stream_delay_ms as i32,
                                    stream_sync_start_ms: c.stream_sync_start_ms as i32,
                                })
                        })
                        .collect(),
                    current_map: {
                        let level = m.selected_level.as_ref();
                        if let Some(level) = level { 
                            Some(Map {
                                id: level.level_id.clone(),
                                name: level.name.clone(),
                                difficulty: m.selected_difficulty,
                                modifiers: vec![],
                            })
                        } else {
                            None
                        }
                    },
                    scores: m
                        .associated_users
                        .iter()
                        .filter(|u| {
                            self.players
                                .iter()
                                .map(|p| p.guid.clone())
                                .collect::<Vec<String>>()
                                .contains(u)
                        })
                        .filter(|u| self.rts.contains_key(*u))
                        .map(|u| {
                            let rts = self.rts.get(u).unwrap();
                            let right_hand = rts.right_hand.clone().unwrap();
                            let left_hand = rts.left_hand.clone().unwrap();
                            Score {
                                owner_id: Uuid::parse_str(u).unwrap(),
                                score: rts.score,
                                score_with_modifiers: rts.score_with_modifiers,
                                max_score: rts.max_score,
                                max_score_with_modifiers: rts.max_score_with_modifiers,
                                combo: rts.combo,
                                player_health: rts.player_health as f64,
                                accuracy: rts.accuracy as f64,
                                song_position: rts.song_position as f64,
                                notes_missed: rts.notes_missed,
                                bad_cuts: rts.bad_cuts,
                                bomb_hits: rts.bomb_hits,
                                wall_hits: rts.wall_hits,
                                max_combo: rts.max_combo,
                                left_hand_hits: left_hand.hit,
                                left_hand_misses: left_hand.miss,
                                left_hand_bad_cut: left_hand.bad_cut,
                                right_hand_hits: right_hand.hit,
                                right_hand_misses: right_hand.miss,
                                right_hand_bad_cut: right_hand.bad_cut,
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
