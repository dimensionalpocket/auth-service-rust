CREATE TABLE sites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_ts INTEGER NOT NULL,
    updated_ts INTEGER NOT NULL,
    
    -- The site slug, defined by the user, used in URLs.
    slug TEXT UNIQUE NOT NULL,

    -- The subdomain. Can be NULL if the site is a bare domain (e.g., mysite.com).
    subdomain TEXT,

    -- The site port, if applicable.
    port INTEGER,

    -- The protocol used by the site (http or https). Default is 'https'.
    protocol TEXT NOT NULL DEFAULT 'https',

    metadata_json TEXT
);
