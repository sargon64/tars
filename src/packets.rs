use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    proto::{
        models,
        packet::{self, event},
    }, connection::TAConnection,
    // TA_CON,
};

#[derive(Debug, Default, Clone)]
pub struct TAState {
    pub server_users: Vec<models::User>,
    pub coordinators: Vec<models::User>,
    pub players: Vec<models::User>,
    pub matches: Vec<models::Match>,
    pub servers: Vec<models::CoreServer>,
    pub rts: HashMap<String, models::RealtimeScore>, // perhaps 
}

impl TAState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn process_event(&mut self, event: packet::Event) -> anyhow::Result<()> {
        if let Some(obj) = event.changed_object {
                match obj {
                    event::ChangedObject::UserAddedEvent(e) => {
                        match e.user {
                            Some(user) => {
                                // unsafe: transmute_copy is safe because user.client_type always contains a valid value
                                match unsafe { std::mem::transmute_copy(&user.client_type) } {
                                    models::user::ClientTypes::Coordinator => {
                                        log::info!("Coordinator added: {}", user.name);
                                        self.coordinators.push(user);
                                    }
                                    models::user::ClientTypes::Player => {
                                        log::info!("Player added: {}", user.name);
                                        self.players.push(user)
                                    }
                                    _ => {}
                                }
                            }
                            None => {
                                log::warn!("Received UserAddedEvent with no user");
                            }
                        }
                    }
                    event::ChangedObject::UserUpdatedEvent(e) => {
                        match e.user {
                            Some(user) => {
                                // unsafe: transmute_copy is safe because user.client_type always contains a valid value
                                match unsafe { std::mem::transmute_copy(&user.client_type) } {
                                    models::user::ClientTypes::Coordinator => {
                                        log::info!("Coordinator updated: {}", user.name);
                                        self.coordinators
                                            .iter_mut()
                                            .find(|u| u.guid == user.guid)
                                            .map(|u| *u = user);
                                    }
                                    models::user::ClientTypes::Player => {
                                        log::info!("Player updated: {}", user.name);
                                        self.players
                                            .iter_mut()
                                            .find(|u| u.guid == user.guid)
                                            .map(|u| *u = user);
                                    }
                                    _ => {}
                                }
                            }
                            None => {
                                log::warn!("Received UserUpdatedEvent with no user");
                            }
                        }
                    }
                    event::ChangedObject::UserLeftEvent(e) => {
                        match e.user {
                            Some(user) => {
                                // unsafe: transmute_copy is safe because user.client_type always contains a valid value
                                match unsafe { std::mem::transmute_copy(&user.client_type) } {
                                    models::user::ClientTypes::Coordinator => {
                                        log::info!("Coordinator left: {}", user.name);
                                        self.coordinators.retain(|u| u.guid != user.guid);
                                    }
                                    models::user::ClientTypes::Player => {
                                        log::info!("Player left: {}", user.name);
                                        self.players.retain(|u| u.guid != user.guid);
                                    }
                                    _ => {}
                                }
                            }
                            None => {
                                log::warn!("Received UserLeftEvent with no user");
                            }
                        }
                    }
                    event::ChangedObject::MatchCreatedEvent(e) => {
                        match e.r#match {
                            Some(mut r#match) => {
                                log::info!("Match created: {}", r#match.guid);
                                //add the overlay to the match's associated users.
                                r#match
                                    .associated_users
                                    .extend(self.server_users.iter().filter(|f| !f.name.contains("TX")).map(|u| u.guid.clone()));

                                self.matches.push(r#match.clone());

                                let mut con = TAConnection::connect(
                                    std::env::var("TA_WS_URI").unwrap(),
                                    "TA-Relay-TX",
                                )
                                .await
                                .unwrap();

                                con
                                    // .write()
                                    // .await
                                    // .as_mut()
                                    // .unwrap()
                                    .send(packet::Packet {
                                        id: Uuid::new_v4().to_string(),
                                        from: "".to_string(),
                                        packet: Some(packet::packet::Packet::Event(
                                            packet::Event {
                                                changed_object: Some(
                                                    event::ChangedObject::MatchUpdatedEvent(
                                                        event::MatchUpdatedEvent {
                                                            r#match: Some(r#match),
                                                        },
                                                    ),
                                                ),
                                            },
                                        )),
                                    })
                                    .await?;
                                    
                                con.close().await;
                            }
                            None => {
                                log::warn!("Received MatchCreatedEvent with no match");
                            }
                        }
                    }
                    event::ChangedObject::MatchUpdatedEvent(e) => match e.r#match {
                        Some(r#match) => {
                            log::info!("Match updated: {}", r#match.guid);
                            self.matches
                                .iter_mut()
                                .find(|m| m.guid == r#match.guid)
                                .map(|m| *m = r#match);
                        }
                        None => {
                            log::warn!("Received MatchUpdatedEvent with no match");
                        }
                    },
                    event::ChangedObject::MatchDeletedEvent(e) => match e.r#match {
                        Some(r#match) => {
                            log::info!("Match deleted: {}", r#match.guid);
                            self.matches.retain(|m| m.guid != r#match.guid);
                        }
                        None => {
                            log::warn!("Received MatchDeletedEvent with no match");
                        }
                    },
                    event::ChangedObject::QualifierCreatedEvent(_) => todo!(),
                    event::ChangedObject::QualifierUpdatedEvent(_) => todo!(),
                    event::ChangedObject::QualifierDeletedEvent(_) => todo!(),
                    event::ChangedObject::HostAddedEvent(e) => match e.server {
                        Some(host) => {
                            log::info!("Host added: {}", host.name);
                            self.servers.push(host);
                        }
                        None => {
                            log::warn!("Received HostAddedEvent with no host");
                        }
                    },
                    event::ChangedObject::HostDeletedEvent(e) => match e.server {
                        Some(host) => {
                            log::info!("Host deleted: {}", host.name);
                            self.servers.retain(|h| h.name != host.name);
                        }
                        None => {
                            log::warn!("Received HostDeletedEvent with no host");
                        }
                    },
                }
            };
        Ok(())
    }

    pub async fn process_response(&mut self, event: packet::Response) -> anyhow::Result<()> {
        match event.details {
            Some(e) => {
                match e {
                    packet::response::Details::Connect(c) => {
                        match c.state {
                            Some(state) => {
                                log::info!(
                                    "Connected to server: {}",
                                    state.server_settings.unwrap_or_default().server_name
                                );
                                // unwrap: there will always be a server user, as to receive a connect response, the server must have a user
                                let server_users = state
                                    .users
                                    .iter()
                                    .filter(|u| {
                                        u.client_type
                                            == models::user::ClientTypes::WebsocketConnection as i32
                                    })
                                    .cloned()
                                    .collect();
                                let coordinators = state
                                    .users
                                    .iter()
                                    .filter(|u| {
                                        u.client_type
                                            == models::user::ClientTypes::Coordinator as i32
                                    })
                                    .cloned()
                                    .collect();
                                let players = state
                                    .users
                                    .iter()
                                    .filter(|u| {
                                        u.client_type == models::user::ClientTypes::Player as i32
                                    })
                                    .cloned()
                                    .collect();
                                let matches = state.matches;
                                let servers = state.known_hosts;
                                self.server_users = server_users;
                                self.coordinators = coordinators;
                                self.players = players;
                                self.matches = matches;
                                self.servers = servers;
                            }
                            None => {
                                log::warn!("Received Connect response with no state");
                            }
                        }
                    }
                    packet::response::Details::LeaderboardScores(_) => todo!(),
                    packet::response::Details::LoadedSong(_) => todo!(),
                    packet::response::Details::Modal(_) => todo!(),
                    packet::response::Details::ModifyQualifier(_) => todo!(),
                    packet::response::Details::ImagePreloaded(_) => todo!(),
                }
            }
            None => {
                log::warn!("Received Response with no details");
            }
        }
        Ok(())
    }

    pub async fn process_push(&mut self, push: packet::Push) -> anyhow::Result<()> {
        match push.data {
            Some(data) => match data {
                packet::push::Data::RealtimeScore(s) => {
                    log::info!("Received RealtimeScore of {} for {}", &s.score, &s.user_guid);
                    let user = self.players.iter().find(|u| u.guid == s.user_guid).unwrap();
                    let _ = tokio::fs::create_dir_all(format!("./data/{}", &user.name)).await;
                    let _ = tokio::fs::write(
                        format!(
                            "./data/{}/{}.dat",
                            user.name,
                            chrono::Utc::now().timestamp_millis()
                        ),
                        format!("{:#?}", &s),
                    )
                    .await;

                    self.rts.insert(s.user_guid.clone(), s);
                }
                packet::push::Data::LeaderboardScore(_) => todo!(),
                packet::push::Data::SongFinished(s) => {
                    let player = s.player.unwrap();
                    log::info!(
                        "Received SongFinished for {}, their final score was {:#?}",
                        player.name,
                        self.rts.get(player.guid.as_str()).unwrap()
                    );
                }
            },
            None => {
                log::warn!("Received Push with no data");
            }
        }
        Ok(())
    }
}

pub async fn route_packet(state: &mut TAState, packet: packet::Packet) -> anyhow::Result<()> {
    log::debug!("Received packet: {:?}", packet.packet);
    match packet.packet {
        Some(packet::packet::Packet::Event(p)) => {
            state.process_event(p).await?;
        }
        Some(packet::packet::Packet::Response(p)) => {
            state.process_response(p).await?;
        }
        Some(packet::packet::Packet::Push(p)) => {
            state.process_push(p).await?;
        }
        None => {}
        _ => {
            log::warn!(
                "Received unhandled packet type: {}",
                type_of(&packet.packet.unwrap())
            );
        }
    }
    Ok(())
}

#[inline]
fn type_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}
