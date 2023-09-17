-- Add migration script here
create table users (
    id uuid primary key not null default gen_random_uuid(),
    steam_id text not null,
    name text not null
);

create table real_time_score(
    owner uuid not null references users(id),
    time timestamptz not null default now(),
    score int not null,
    score_with_modifiers int not null,
    max_score int not null,
    max_score_with_modifiers int not null,
    combo int not null,
    player_health float not null,
    accuracy float not null,
    song_position float not null,
    notes_missed int not null,
    bad_cuts int not null,
    bomb_hits int not null,
    wall_hits int not null,
    max_combo int not null,
    left_hand_hits int not null,
    left_hand_misses int not null,
    left_hand_bad_cut int not null,
    right_hand_hits int not null,
    right_hand_misses int not null,
    right_hand_bad_cut int not null
);

select create_hypertable('real_time_score', 'time');
