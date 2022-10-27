CREATE USER computer WITH SUPERUSER ENCRYPTED PASSWORD 'my-strong-computer-pwd';

CREATE TABLE IF NOT EXISTS companies (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE CHECK (name <> '')
);

CREATE TYPE user_category AS ENUM ('user', 'staff', 'admin');

CREATE TYPE web_access_enum AS ENUM ('blocked', 'readonly', 'full');

CREATE TYPE api_access_enum AS ENUM ('blocked', 'readonly', 'full');

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  uuid SERIAL2 UNIQUE,
  category user_category NOT NULL,
  company TEXT NOT NULL REFERENCES companies ON DELETE CASCADE,
  first_name TEXT NOT NULL CHECK (first_name <> ''),
  last_name TEXT NOT NULL CHECK (last_name <> ''),
  pwdhash TEXT NOT NULL CHECK (pwdhash <> ''),
  email TEXT NOT NULL UNIQUE CHECK (email <> ''),
  last_active INT4 NOT NULL CHECK (last_active >= 0),
  web_access web_access_enum NOT NULL,
  api_access api_access_enum NOT NULL,
  UNIQUE (first_name, last_name)
);

CREATE TYPE presentation_units AS ENUM ('us', 'eu');
CREATE TYPE computation_mode AS ENUM ('off', 'client', 'server');

CREATE TABLE IF NOT EXISTS wells (
  id TEXT PRIMARY KEY,
  uuid SERIAL2 UNIQUE,
  name TEXT NOT NULL UNIQUE CHECK (name <> ''),
  company TEXT NOT NULL REFERENCES companies ON DELETE CASCADE,
  initial_reservoir_pressure FLOAT4 NULL CHECK (initial_reservoir_pressure > 0),
  pressure_sensors_height FLOAT4 NULL CHECK (pressure_sensors_height > 0),
  units presentation_units NOT NULL,
  bhp_mode computation_mode NOT NULL,
  bht_mode computation_mode NOT NULL,
  whp_mode computation_mode NOT NULL,
  rate_mode computation_mode NOT NULL,
  rho_mode computation_mode NOT NULL,
  vtot_mode computation_mode NOT NULL,
  ii_mode computation_mode NOT NULL,
  computer_needed BOOLEAN NOT NULL DEFAULT FALSE,
  computed_to INT4 NOT NULL CHECK (computed_to >= 0)
);

CREATE TYPE point_value_size AS ENUM ('f32', 'f64');

CREATE TABLE IF NOT EXISTS custom_tags (
  well INT2 NOT NULL REFERENCES wells(uuid) ON DELETE CASCADE,
  id INT2 NOT NULL CHECK (id > 7),
  value_size point_value_size NOT NULL,
  units_text TEXT NOT NULL CHECK (units_text <> ''),
  name TEXT NOT NULL CHECK (name <> ''),
  description TEXT NOT NULL CHECK (description <> ''),
  PRIMARY KEY (well, id),
  UNIQUE (well, name)
);

CREATE FUNCTION on_custom_tag_delete() RETURNS TRIGGER AS $$
  BEGIN
    IF (OLD.value_size = 'f32') THEN
      DELETE FROM public.points_f32 WHERE well = OLD.well AND tag = OLD.id;
      DELETE FROM public.points_f32 WHERE well = OLD.well AND tag = -OLD.id;
    ELSIF (OLD.value_size = 'f64') THEN
      DELETE FROM public.points_f64 WHERE well = OLD.well AND tag = OLD.id;
      DELETE FROM public.points_f64 WHERE well = OLD.well AND tag = -OLD.id;
    END IF;
    RETURN OLD;
  END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER on_custom_tag_delete_trigger BEFORE DELETE ON public.custom_tags
  FOR EACH ROW EXECUTE FUNCTION on_custom_tag_delete();

CREATE FUNCTION next_custom_tagid(wellarg INT2) RETURNS INT2 AS $$
  SELECT COALESCE(MAX(id)+1,8) AS result FROM public.custom_tags WHERE well = wellarg;
$$ LANGUAGE SQL;

CREATE TABLE IF NOT EXISTS points_f32 (
  well INT2 NOT NULL REFERENCES wells(uuid) ON DELETE CASCADE,
  tag INT2 NOT NULL CHECK (tag != 0),
  time INT4 NOT NULL CHECK (time > 0),
  value FLOAT4 NOT NULL,
  PRIMARY KEY (well, tag, time)
);

CREATE TABLE IF NOT EXISTS points_f64 (
  well INT2 NOT NULL REFERENCES wells(uuid) ON DELETE CASCADE,
  tag INT2 NOT NULL CHECK (tag != 0),
  time INT4 NOT NULL CHECK (time > 0),
  value FLOAT8 NOT NULL,
  PRIMARY KEY (well, tag, time)
);

CREATE TYPE cycle_status AS ENUM ('uncommitted', 'baddata', 'committed');

CREATE TYPE cycle_last_rate AS (
  time INT4,
  value FLOAT4
);

CREATE TYPE cycle_isip AS (
  time INT4,
  lower_value FLOAT4,
  upper_value FLOAT4 
);

CREATE TYPE cycle_horner AS (
  value FLOAT4,
  x1 FLOAT8,
  y1 FLOAT4,
  x2 FLOAT8,
  y2 FLOAT4
);

CREATE TYPE cycle_stiffness AS (
  timeshift FLOAT4,
  rate_time_ms FLOAT8,
  bhp_time_ms FLOAT8
);

CREATE TABLE IF NOT EXISTS cycles (
  well INT2 NOT NULL REFERENCES wells(uuid) ON DELETE CASCADE,
  id INT2 NOT NULL CHECK (id > 0),
  status cycle_status NOT NULL,
  t1 INT4 NOT NULL CHECK (t1 > 0),
  t2 INT4 NOT NULL CHECK (t2 > 0),
  t3 INT4 NOT NULL CHECK (t3 > 0),
  last_action_by INT2 NOT NULL REFERENCES users(uuid) ON DELETE RESTRICT,
  batch_volume FLOAT4 NULL CHECK (batch_volume >= 0),
  total_volume FLOAT8 NULL CHECK (total_volume >= 0),
  min_bhp FLOAT4 NULL CHECK (min_bhp > 0),
  max_bhp FLOAT4 NULL CHECK (max_whp > 0),
  min_whp FLOAT4 NULL CHECK (min_whp >= 0),
  max_whp FLOAT4 NULL CHECK (max_whp >= 0),
  min_bht FLOAT4 NULL CHECK (min_bht > 0),
  max_bht FLOAT4 NULL CHECK (max_bht > 0),
  avg_rate FLOAT4 NULL CHECK (avg_rate >= 0),
  max_rate FLOAT4 NULL CHECK (max_rate >= 0),
  max_rho FLOAT4 NULL CHECK (max_rho > 0),
  end_rho FLOAT4 NULL CHECK (end_rho > 0),
  min_ii FLOAT4 NULL CHECK (min_ii > 0),
  avg_ii FLOAT4 NULL CHECK (avg_ii > 0),
  max_ii FLOAT4 NULL CHECK (max_ii > 0),
  last_rate cycle_last_rate NULL,
  isip_bhp cycle_isip NULL,
  isip_whp cycle_isip NULL,
  waterhammer_bhp_endto INT4 NULL CHECK (waterhammer_bhp_endto > 0),
  waterhammer_whp_endto INT4 NULL CHECK (waterhammer_whp_endto > 0),
  horner_bhp cycle_horner NULL,
  horner_whp cycle_horner NULL,
  horner_bht cycle_horner NULL,
  stiffness cycle_stiffness NULL,
  PRIMARY KEY (well, id)
);

CREATE FUNCTION next_cycleid(wellarg INT2) RETURNS INT2 AS $$
  SELECT COALESCE(MAX(id)+1,1) AS result FROM public.cycles WHERE well = wellarg;
$$ LANGUAGE SQL;

CREATE TYPE fourier_category AS ENUM ('bhp', 'whp');

CREATE TABLE IF NOT EXISTS fourier (
  well INT2 NOT NULL REFERENCES wells(uuid) ON DELETE CASCADE,
  cycle INT2 NOT NULL CHECK (cycle > 0),
  category fourier_category,
  id INT2 NOT NULL CHECK (id > 0),
  x FLOAT4 NOT NULL CHECK (x > 0),
  y FLOAT4 NOT NULL CHECK (y > 0),
  PRIMARY KEY (well, cycle, category, id),
  FOREIGN KEY (well, cycle) REFERENCES cycles(well, id) ON DELETE CASCADE
);

CREATE FUNCTION next_fourier_bhp_id(wellarg INT2, cyclearg INT2)
      RETURNS INT2 AS $$
  SELECT COALESCE(MAX(id)+1,1) AS result FROM public.fourier
  WHERE well = wellarg AND cycle = cyclearg AND category = 'bhp';
$$ LANGUAGE SQL;

CREATE FUNCTION next_fourier_whp_id(wellarg INT2, cyclearg INT2)
      RETURNS INT2 AS $$
  SELECT COALESCE(MAX(id)+1,1) AS result FROM public.fourier
  WHERE well = wellarg AND cycle = cyclearg AND category = 'whp';
$$ LANGUAGE SQL;

CREATE TYPE line_f32_f32 AS (
  x1 FLOAT4,
  y1 FLOAT4,
  x2 FLOAT4,
  y2 FLOAT4
);

CREATE TABLE IF NOT EXISTS stiffness (
  well INT2 NOT NULL REFERENCES wells(uuid) ON DELETE CASCADE,
  cycle INT2 NOT NULL CHECK (cycle > 0),
  id INT2 NOT NULL CHECK (id > 0),
  value FLOAT4 NOT NULL CHECK (value > 0),
  line_a line_f32_f32 NOT NULL,
  line_b line_f32_f32 NOT NULL,
  PRIMARY KEY (well, cycle, id),
  FOREIGN KEY (well, cycle) REFERENCES cycles(well, id) ON DELETE CASCADE
);

CREATE FUNCTION next_stiffness_id(wellarg INT2, cyclearg INT2)
    RETURNS INT2 AS $$
  SELECT COALESCE(MAX(id)+1,1) AS result
  FROM public.stiffness WHERE well = wellarg AND cycle = cyclearg;
$$ LANGUAGE SQL;

INSERT INTO companies VALUES ('geomec', 'Geomec Engineering');

INSERT INTO users (id,category,company,first_name,last_name,pwdhash,email,
last_active,web_access,api_access) VALUES
 ('vadim', 'admin', 'geomec', 'Vadim', 'Tambovtsev',
    '$argon2id$v=19$m=4096,t=3,p=1$iCTrBRGILv3w1G4x9MdEGw$YNGNG+PzJGiRMG/xV3htsk3BAbU5t8jm+wAeOn31XXE',
    'some-admin@gmail.com', 0, 'full', 'full');

