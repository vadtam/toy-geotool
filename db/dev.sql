INSERT INTO companies VALUES
 ('company1', 'Company 1'),
 ('company2', 'Company 2');

INSERT INTO users (id,category,company,first_name,last_name,pwdhash,email,
last_active,web_access,api_access) VALUES
 ('johntravolta', 'admin', 'geomec', 'John', 'Travolta',
    '$argon2id$v=19$m=4096,t=3,p=1$/wqs8x1dM2JPO+FgNQHzgQ$UOKmM19hFVQOn2EMzUUHsXUggk1BnFVpWgdslXG65lY',
    'john@gmail.com', 0, 'full', 'full'),
 ('enniomorrocone', 'user', 'company1', 'Ennio', 'Morricone',
   '$argon2id$v=19$m=4096,t=3,p=1$/wqs8x1dM2JPO+FgNQHzgQ$UOKmM19hFVQOn2EMzUUHsXUggk1BnFVpWgdslXG65lY',
   'ennio@gmail.com', 0, 'full', 'full');

INSERT INTO wells (id,name,company,initial_reservoir_pressure,pressure_sensors_height,
units,bhp_mode,bht_mode,whp_mode,rate_mode,rho_mode,vtot_mode,ii_mode,
computer_needed,computed_to) VALUES
  ('well1', 'My Well 1', 'company1', NULL, 6735.5, 'us',
      'off', 'client', 'client', 'client', 'server', 'server', 'server', FALSE, 0),
  ('well2', 'My Well 2', 'company1', 3348, 2694.4, 'us',
      'client', 'client', 'client', 'client', 'server', 'server', 'server', FALSE, 0),
  ('well3', 'My Well 3', 'company2', 3450, 4264.65, 'eu',
      'client', 'client', 'client', 'client', 'server', 'server', 'server', FALSE, 0);

