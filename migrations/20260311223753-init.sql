create table if not exists documents (
  id uuid primary key default gen_random_uuid(),
  path text not null,
  created_at timestamp not null default now(),
  updated_at timestamp not null default now(),
  deleted_at timestamp
)