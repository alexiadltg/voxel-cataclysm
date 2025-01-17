use super::{ClientLobby, NetworkMapping};
use crate::{
    voxel::{
        animation::Animations,
        loading::MyAssets,
        mob::Mob,
        networking::{ControlledPlayer, ControlledPlayerCollider, PlayerInfo},
        player::{
            bundle::{BasePlayerBundle, MyCamera3dBundle, PlayerColliderBundle, PlayerHeadBundle},
            Body, MobSpawnTimer,
        },
        AttackWanted, Attacked,
    },
    GameState,
};
use bevy::{prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::{ActiveEvents, Collider, GravityScale, RigidBody};
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use common::{
    ChatMessage, ClientChannel, DisplayMessage, MobSend, NetworkedEntities, Player, PlayerCommand,
    ServerChannel, ServerMessages,
};

#[derive(Component)]
pub struct NetworkMob(pub String);

#[allow(clippy::too_many_arguments)]
fn sync_players(
    mut cmds: Commands,
    mut client: ResMut<RenetClient>,
    transport: Res<NetcodeClientTransport>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    my_assets: Res<MyAssets>,
    mut queries: ParamSet<(
        Query<&Transform>,
        Query<&ControlledPlayer>,
        Query<&Mob>,
        Query<(&mut Transform, &NetworkMob)>,
        Query<(Entity, &Mob)>,
        Query<(Entity, &NetworkMob)>,
    )>,
    mut display_message: ResMut<DisplayMessage>,
) {
    let client_id = transport.client_id();
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                entity,
                translation,
            } => {
                println!("Player {} connected.", id);
                let mut map = HashMap::new();
                map.insert("walk".to_string(), my_assets.player_animation_walk.clone());
                map.insert("hit".to_string(), my_assets.player_animation_hit.clone());

                let mut client_entity = cmds.spawn((
                    BasePlayerBundle::default(),
                    Animations(map),
                    TransformBundle {
                        local: Transform::from_xyz(translation[0], translation[1], translation[2])
                            .looking_to(Vec3::Z, Vec3::Y),
                        ..default()
                    },
                ));
                if client_id == id {
                    client_entity
                        .insert(ControlledPlayer)
                        .insert(Player { id })
                        .insert(MobSpawnTimer {
                            get_timer: Timer::from_seconds(5.0, TimerMode::Once),
                            current_mobs: 0,
                            max_mobs: 30,
                        })
                        .with_children(|player| {
                            player.spawn(Body).insert(SceneBundle {
                                scene: my_assets.player.clone(),
                                transform: Transform::IDENTITY.looking_to(Vec3::Z, Vec3::Y),
                                ..default()
                            });
                            player
                                .spawn(PlayerColliderBundle::default())
                                .insert(ControlledPlayerCollider);

                            player
                                .spawn(PlayerHeadBundle::default())
                                .with_children(|head| {
                                    head.spawn(MyCamera3dBundle::default());
                                });
                        });
                } else {
                    client_entity.with_children(|player| {
                        player.spawn(SceneBundle {
                            scene: my_assets.player.clone(),
                            transform: Transform::IDENTITY.looking_to(Vec3::Z, Vec3::Y),
                            ..default()
                        });
                    });
                }
                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity: client_entity.id(),
                };
                lobby.players.insert(id, player_info);
                network_mapping.0.insert(entity, client_entity.id());
            }
            ServerMessages::PlayerRemove { id } => {
                println!("Player {} disconnected.", id);
                if let Some(PlayerInfo {
                    server_entity,
                    client_entity,
                }) = lobby.players.remove(&id)
                {
                    cmds.entity(client_entity).despawn();
                    network_mapping.0.remove(&server_entity);
                }
            }
        }
    }
    // si peta aqui es culpa de l'Alexia
    while let Some(message) = client.receive_message(ServerChannel::Host) {
        let host = bincode::deserialize(&message).unwrap();
        if host {
            println!("I'm the host");
        } else {
            println!("I'm not the host");
        }
    }
    while let Some(message) = client.receive_message(ServerChannel::NonNetworkedEntities) {
        let mob: MobSend = bincode::deserialize(&message).unwrap();
        let mut flag = false;
        //println!("mob {:?}", mob);
        for id in queries.p2().iter().map(|mob| &mob.0) {
            // if id equals mob id
            if id == &mob.id {
                flag = true;
                break;
            }
        }
        for (mut transform, id) in queries.p3().iter_mut() {
            if id.0 == mob.id {
                transform.translation = mob.translation;
                flag = true;
                break;
            }
        }
        if !flag {
            cmds.spawn((
                Collider::cuboid(1.0, 1.0, 1.0),
                SceneBundle {
                    scene: my_assets.slime.clone(),
                    transform: Transform::from_translation(mob.translation)
                        .looking_to(Vec3::Z, Vec3::Y),
                    ..default()
                },
            ))
            .insert(RigidBody::Dynamic)
            .insert(GravityScale(0.0))
            .insert(ActiveEvents::COLLISION_EVENTS)
            .insert(NetworkMob(mob.id.clone()));
        }
    }
    while let Some(message) = client.receive_message(ServerChannel::NetworkedEntities) {
        let networked_entities: NetworkedEntities = bincode::deserialize(&message).unwrap();
        for i in 0..networked_entities.entities.len() {
            if let Some(entity) = network_mapping.0.get(&networked_entities.entities[i]) {
                // if the entity is the ControlledPlayer, we don't want to apply it
                if queries.p1().get(*entity).is_err() {
                    if let Ok(current_transform) = queries.p0().get(*entity) {
                        let translation = networked_entities.translations[i].into();
                        let rotation = networked_entities.rotations[i];
                        if translation != current_transform.translation {
                            let transform = Transform {
                                rotation,
                                translation,
                                ..Default::default()
                            };
                            cmds.entity(*entity).insert(transform);
                        }
                    }
                }
            }
        }
    }
    while let Some(message) = client.receive_message(ServerChannel::MobAttacked) {
        let sent_id: String = bincode::deserialize(&message).unwrap();
        for (entity, id) in queries.p4().iter() {
            // if id equals mob id he's the one who was attacked
            if id.0 == sent_id {
                println!("Mob {} was attacked", &id.0.clone());
                cmds.entity(entity).insert(Attacked { damage: 10 });
            }
        }
        for (entity, id) in queries.p5().iter() {
            // if id equals mob id he's the one who was attacked
            if id.0 == sent_id {
                println!("Mob {} was attacked, -10hp", &id.0.clone());
                cmds.entity(entity).insert(Attacked { damage: 10 });
            }
        }
    }
    while let Some(message) = client.receive_message(ServerChannel::ChatChannel) {
        let textmess: ChatMessage = bincode::deserialize(&message).unwrap();
        println!("Received message: {:?}", textmess.message);
        display_message.message = textmess.message.clone();
    }
}

pub fn send_one_chat(
    chat_messages: ResMut<ChatMessage>,
    player_id: Query<&Player>,
    mut client: ResMut<RenetClient>,
) {
    if player_id.get_single().is_err() {
        return;
    }
    if chat_messages.message.is_empty() {
    } else {
        let message = ChatMessage {
            client_id: player_id.get_single().unwrap().id,
            message: chat_messages.message.clone(),
        };
        if !(client.is_disconnected()) {
            let message = bincode::serialize(&message).unwrap();
            client.send_message(ClientChannel::Chat, message);
        }
        println!("Sending message: {:?}", message);
    }
}

fn sync_input(
    player_input: Query<&Transform, With<ControlledPlayer>>,
    mut client: ResMut<RenetClient>,
) {
    if player_input.get_single().is_err() {
        return;
    }
    let translation = player_input.single();
    let message = bincode::serialize(&translation.translation).unwrap();
    client.send_message(ClientChannel::Input, message)
}

fn sync_rotation(body_rot: Query<&Transform, With<Body>>, mut client: ResMut<RenetClient>) {
    if body_rot.get_single().is_err() {
        return;
    }
    let rotation = body_rot.single();
    let message = bincode::serialize(&rotation.rotation).unwrap();
    client.send_message(ClientChannel::Rots, message)
}

fn sync_player_commands(
    mut player_commands: EventReader<PlayerCommand>,
    mut client: ResMut<RenetClient>,
) {
    for command in player_commands.iter() {
        let command_message = bincode::serialize(command).unwrap();
        client.send_message(ClientChannel::Command, command_message);
    }
}

fn send_text(mut client: ResMut<RenetClient>, mut chat_message: ResMut<ChatMessage>) {
    if chat_message.message.is_empty() {
        return;
    }
    let message = bincode::serialize(&(&chat_message.message, &chat_message.client_id)).unwrap();
    client.send_message(ClientChannel::Chat, message);

    //Reiniciem els valors de chat_message per a que no es repeteixin els missatges
    //Es una mica chapuzas pero ens queda nomes un dia nois.
    chat_message.message = String::new();
    chat_message.client_id = 0;
}

fn sync_mob_attacked(
    query_p1: Query<(&Mob, Entity), With<AttackWanted>>,
    query_p2: Query<(&NetworkMob, Entity), With<AttackWanted>>,
    mut client: ResMut<RenetClient>,
    mut cmds: Commands,
) {
    for (id, entity) in query_p1.iter() {
        println!("Mob Attacked: {:?}", id.0);
        let message = bincode::serialize(&id.0).unwrap();
        client.send_message(ClientChannel::MobAttacked, message);
        cmds.entity(entity).remove::<AttackWanted>();
    }
    for (id, entity) in query_p2.iter() {
        println!("NetworkMob Attacked: {:?}", id.0);
        let message = bincode::serialize(&id.0).unwrap();
        client.send_message(ClientChannel::MobAttacked, message);
        cmds.entity(entity).remove::<AttackWanted>();
    }
}

fn send_mob(mut client: ResMut<RenetClient>, mob_query: Query<(&Transform, &Mob)>) {
    for (pos, mob) in mob_query.iter() {
        //println!("Pos: {:?} Mob:{:?}", pos, mob);
        let mob_send = MobSend {
            id: mob.0.clone(),
            translation: pos.translation,
        };
        let message = bincode::serialize(&mob_send).unwrap();
        client.send_message(ClientChannel::Mobs, message);
    }
}

pub struct NetSyncPlugin;
impl Plugin for NetSyncPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            (
                sync_rotation,
                sync_input,
                sync_player_commands,
                sync_players,
                send_text,
                send_one_chat,
                send_mob,
                sync_mob_attacked,
            )
                .distributive_run_if(bevy_renet::transport::client_connected)
                .in_set(OnUpdate(GameState::Game)),
        );
    }
}
