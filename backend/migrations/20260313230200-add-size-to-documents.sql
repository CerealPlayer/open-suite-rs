alter table documents
add column if not exists size integer not null default 0;

alter table documents
alter column size drop default;
