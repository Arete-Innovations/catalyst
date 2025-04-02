CREATE TABLE cronjobs (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    timer INT NOT NULL,      
    status VARCHAR NOT NULL, 
    last_run BIGINT
);
