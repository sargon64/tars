use async_graphql::{Enum, InputObject, SimpleObject};
use tap::Tap;
use tracing::warn;
use uuid::Uuid;

use crate::{packets::TAState, parse_uuid};

#[derive(SimpleObject, Default)]
pub struct User {
    guid: Uuid,
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
#[derive(Enum, Default, Clone, Copy, Eq, PartialEq)]
pub enum PlayState {
    #[default]
    Waiting = 0,
    InGame = 1,
}

#[repr(i32)]
#[derive(Enum, Default, Clone, Copy, Eq, PartialEq)]
pub enum DownloadState {
    #[default]
    None = 0,
    Downloading = 1,
    Downloaded = 2,
    DownloadError = 3,
}

#[derive(SimpleObject, Default, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct Team {
    guid: Uuid,
    name: String,
}

#[derive(SimpleObject)]
pub struct Match {
    guid: Uuid,
    players: Vec<User>,
    teams: Vec<Team>,
    coordinators: Vec<User>,
    current_map: Option<Map>,
    scores: Vec<Score>,
}

#[derive(SimpleObject)]
pub struct Map {
    hash: String,
    name: String,
    difficulty: i32,
    modifiers: Vec<String>,
}

#[derive(SimpleObject)]
pub struct Score {
    owner_guid: Uuid,

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

#[derive(SimpleObject)]
pub struct GQLTAState {
    pub coordinators: Vec<User>,
    pub players: Vec<User>,
    pub matches: Vec<Match>,
}

#[derive(SimpleObject, Clone)]
pub struct Page {
    pub data: Vec<PageData>,
    pub path: String,
    pub path_name: String,
}

#[derive(SimpleObject, Clone)]
pub struct PageData {
    pub key: String,
    pub value: String,
}

#[derive(InputObject, Clone)]
pub struct InputPage {
    pub data: Vec<InputPageData>,
    pub path: String,
    pub path_name: String,
}

#[derive(InputObject, Clone)]
pub struct InputPageData {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
pub struct GQLOverState {
    pub page: Page,
}

impl Default for GQLOverState {
    fn default() -> Self {
        Self {
            page: Page {
                path: "/".to_string(),
                data: vec![],
                path_name: "root".to_string(),
            },
        }
    }
}

impl InputPage {
    pub fn into_page(self) -> Page {
        Page {
            path: self.path,
            data: self
                .data
                .into_iter()
                .map(|f| PageData {
                    key: f.key,
                    value: f.value,
                })
                .collect(),
            path_name: self.path_name,
        }
    }
}

impl TAState {
    pub async fn get_single_match_gql(&self, id: Uuid) -> anyhow::Result<Option<Match>> {
        let match_ = match self.matches.iter().find(|m| parse_uuid(&m.guid) == id) {
            Some(match_) => match_,
            None => {
                warn!("No match with id {}.", id);
                return Ok(None);
            }
        };
        Ok(Some(Match {
            guid: parse_uuid(&match_.guid),
            players: match_
                .associated_users
                .iter()
                .filter_map(|u| {
                    self.players.iter().find(|p| p.guid == *u).map(|p| User {
                        guid: parse_uuid(&p.guid),
                        name: p.name.clone(),
                        user_id: p.user_id.clone(),
                        play_state: unsafe {
                            std::mem::transmute_copy::<i32, crate::structs::PlayState>(
                                &p.play_state,
                            )
                        },
                        download_state: unsafe {
                            std::mem::transmute_copy::<i32, crate::structs::DownloadState>(
                                &p.download_state,
                            )
                        },
                        team: p.team.as_ref().map(|t| Team {
                            guid: parse_uuid(&t.id),
                            name: t.name.clone(),
                        }),
                        mod_list: p.mod_list.clone(),
                        stream_delay_ms: p.stream_delay_ms as i32,
                        stream_sync_start_ms: p.stream_sync_start_ms as i32,
                    })
                })
                .collect(),
            teams: match_
                .associated_users
                .iter()
                .filter_map(|u| {
                    self.players.iter().find(|p| p.guid == *u).and_then(|p| {
                        p.team.as_ref().map(|t| Team {
                            guid: parse_uuid(&t.id),
                            name: t.name.clone(),
                        })
                    })
                })
                .collect::<Vec<_>>()
                .tap_mut(|v| v.sort())
                .tap_mut(|v| v.dedup()),
            coordinators: match_
                .associated_users
                .iter()
                .filter_map(|u| {
                    self.coordinators
                        .iter()
                        .find(|p| p.guid == *u)
                        .map(|c| User {
                            guid: parse_uuid(&c.guid),
                            name: c.name.clone(),
                            user_id: c.user_id.clone(),
                            play_state: unsafe {
                                std::mem::transmute_copy::<i32, crate::structs::PlayState>(
                                    &c.play_state,
                                )
                            },
                            download_state: unsafe {
                                std::mem::transmute_copy::<i32, crate::structs::DownloadState>(
                                    &c.download_state,
                                )
                            },
                            team: c.team.as_ref().map(|t| Team {
                                guid: parse_uuid(&t.id),
                                name: t.name.clone(),
                            }),
                            mod_list: c.mod_list.clone(),
                            stream_delay_ms: c.stream_delay_ms as i32,
                            stream_sync_start_ms: c.stream_sync_start_ms as i32,
                        })
                })
                .collect(),
            current_map: {
                let level: Option<&crate::proto::models::PreviewBeatmapLevel> =
                    match_.selected_level.as_ref();
                level.map(|level| Map {
                    hash: level
                        .level_id
                        .clone()
                        .split('_')
                        .collect::<Vec<_>>()
                        .last()
                        .unwrap_or(&"")
                        .to_string(),
                    name: level.name.clone(),
                    difficulty: match_.selected_difficulty,
                    modifiers: vec![],
                })
            },
            scores: {
                match_
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
                        let rts = match self.rts.get(u) {
                            Some(rts) => rts,
                            None => {
                                warn!("No RTS for user {}.", u);
                                return Err(anyhow::anyhow!("No RTS for user {}.", u));
                            }
                        };
                        let right_hand = match rts.right_hand.clone() {
                            Some(rts) => rts,
                            None => {
                                warn!("No right hand for user {}.", u);
                                return Err(anyhow::anyhow!("No right hand for user {}.", u));
                            }
                        };
                        let left_hand = match rts.left_hand.clone() {
                            Some(rts) => rts,
                            None => {
                                warn!("No left hand for user {}.", u);
                                return Err(anyhow::anyhow!("No left hand for user {}.", u));
                            }
                        };
                        Ok(Score {
                            owner_guid: parse_uuid(u),
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
                        })
                    })
                    .collect::<anyhow::Result<Vec<Score>>>()?
            },
        }))
    }

    pub async fn into_gql(&self) -> anyhow::Result<GQLTAState> {
        Ok(GQLTAState {
            players: self
                .players
                .iter()
                .map(|p| User {
                    guid: parse_uuid(&p.guid),
                    user_id: p.user_id.clone(),
                    name: p.name.clone(),
                    play_state: unsafe {
                        std::mem::transmute_copy::<i32, crate::structs::PlayState>(&p.play_state)
                    },
                    download_state: unsafe {
                        std::mem::transmute_copy::<i32, crate::structs::DownloadState>(
                            &p.download_state,
                        )
                    },
                    team: p.team.as_ref().map(|t| Team {
                        guid: parse_uuid(&t.id),
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
                    guid: parse_uuid(&p.guid),
                    name: p.name.clone(),
                    user_id: p.user_id.clone(),
                    play_state: unsafe {
                        std::mem::transmute_copy::<i32, crate::structs::PlayState>(&p.play_state)
                    },
                    download_state: unsafe {
                        std::mem::transmute_copy::<i32, crate::structs::DownloadState>(
                            &p.download_state,
                        )
                    },
                    team: p.team.as_ref().map(|t| Team {
                        guid: parse_uuid(&t.id),
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
                .map(|m| {
                    Ok(Match {
                        guid: parse_uuid(&m.guid),
                        players: m
                            .associated_users
                            .iter()
                            .filter_map(|u| {
                                self.players.iter().find(|p| p.guid == *u).map(|p| User {
                                guid: parse_uuid(&p.guid),
                                name: p.name.clone(),
                                user_id: p.user_id.clone(),
                                play_state: unsafe {
                                    std::mem::transmute_copy::<i32, crate::structs::PlayState>(
                                        &p.play_state,
                                    )
                                },
                                download_state: unsafe {
                                    std::mem::transmute_copy::<i32, crate::structs::DownloadState>(
                                        &p.download_state,
                                    )
                                },
                                team: p.team.as_ref().map(|t| Team {
                                    guid: parse_uuid(&t.id),
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
                                self.players.iter().find(|p| p.guid == *u).and_then(|p| {
                                    p.team.as_ref().map(|t| Team {
                                        guid: parse_uuid(&t.id),
                                        name: t.name.clone(),
                                    })
                                })
                            })
                            .collect::<Vec<_>>()
                            .tap_mut(|v| v.sort())
                            .tap_mut(|v| v.dedup()),
                        coordinators: m
                            .associated_users
                            .iter()
                            .filter_map(|u| {
                                self.coordinators
                                        .iter()
                                        .find(|p| p.guid == *u)
                                        .map(|c| User {
                                            guid: parse_uuid(&c.guid),
                                            name: c.name.clone(),
                                            user_id: c.user_id.clone(),
                                            play_state: unsafe {
                                                std::mem::transmute_copy::<
                                                    i32,
                                                    crate::structs::PlayState,
                                                >(
                                                    &c.play_state
                                                )
                                            },
                                            download_state: unsafe {
                                                std::mem::transmute_copy::<
                                                    i32,
                                                    crate::structs::DownloadState,
                                                >(
                                                    &c.download_state
                                                )
                                            },
                                            team: c.team.as_ref().map(|t| Team {
                                                guid: parse_uuid(&t.id),
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
                            level.map(|level| Map {
                                hash: level
                                    .level_id
                                    .clone()
                                    .split('_')
                                    .collect::<Vec<_>>()
                                    .last()
                                    .unwrap_or(&"")
                                    .to_string(),
                                name: level.name.clone(),
                                difficulty: m.selected_difficulty,
                                modifiers: vec![],
                            })
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
                                let rts = match self.rts.get(u) {
                                    Some(rts) => rts,
                                    None => {
                                        warn!("No RTS for user {}.", u);
                                        return Err(anyhow::anyhow!("No RTS for user {}.", u));
                                    }
                                };
                                let right_hand = match rts.right_hand.clone() {
                                    Some(rts) => rts,
                                    None => {
                                        warn!("No right hand for user {}.", u);
                                        return Err(anyhow::anyhow!(
                                            "No right hand for user {}.",
                                            u
                                        ));
                                    }
                                };
                                let left_hand = match rts.left_hand.clone() {
                                    Some(rts) => rts,
                                    None => {
                                        warn!("No left hand for user {}.", u);
                                        return Err(anyhow::anyhow!(
                                            "No left hand for user {}.",
                                            u
                                        ));
                                    }
                                };
                                Ok(Score {
                                    owner_guid: parse_uuid(u),
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
                                })
                            })
                            .collect::<anyhow::Result<Vec<Score>>>()?,
                    })
                })
                .collect::<anyhow::Result<Vec<Match>>>()?,
        })
    }
}
