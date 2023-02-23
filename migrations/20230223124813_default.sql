CREATE TABLE IF NOT EXISTS harp.actions (
    id             serial primary key,
    unique_id      bigint                       not null,
    ip_address     inet                         not null,
    kind           varchar(255)                 not null,
    detail         jsonb,
    created        timestamptz default now()    not null
);