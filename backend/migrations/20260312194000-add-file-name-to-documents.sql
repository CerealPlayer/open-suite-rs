alter table documents
add column if not exists file_name text not null default '';

update documents
set file_name = regexp_replace(path, '^.*/', '')
where file_name = '';

alter table documents
alter column file_name drop default;
