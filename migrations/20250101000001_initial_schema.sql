-- Initial database schema for Rusty Links
-- This migration creates all core tables and relationships

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add index on email for faster lookups
CREATE INDEX idx_users_email ON users(email);

-- Links table
CREATE TABLE links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    domain TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    description TEXT,
    logo BYTEA,
    source_code_url TEXT,
    documentation_url TEXT,
    notes TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived', 'inaccessible', 'repo_unavailable')),
    github_stars INTEGER,
    github_archived BOOLEAN,
    github_last_commit DATE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    refreshed_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT uq_links_user_domain_path UNIQUE (user_id, domain, path)
);

-- Add indexes on links table
CREATE INDEX idx_links_user_id ON links(user_id);
CREATE INDEX idx_links_domain ON links(domain);
CREATE INDEX idx_links_status ON links(status);
CREATE INDEX idx_links_created_at ON links(created_at);

-- Categories table (supports 3-level hierarchy: depth 0, 1, 2)
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    parent_id UUID REFERENCES categories(id) ON DELETE CASCADE,
    depth INTEGER NOT NULL CHECK (depth >= 0 AND depth <= 2),
    sort_order INTEGER,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_categories_user_name UNIQUE (user_id, lower(name))
);

-- Add indexes on categories table
CREATE INDEX idx_categories_user_id ON categories(user_id);
CREATE INDEX idx_categories_parent_id ON categories(parent_id);

-- Languages table
CREATE TABLE languages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_languages_user_name UNIQUE (user_id, lower(name))
);

-- Add index on languages table
CREATE INDEX idx_languages_user_id ON languages(user_id);

-- Licenses table
CREATE TABLE licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL, -- Acronym (e.g., "MIT", "Apache-2.0")
    full_name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_licenses_user_name UNIQUE (user_id, lower(name))
);

-- Add index on licenses table
CREATE INDEX idx_licenses_user_id ON licenses(user_id);

-- Tags table
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_tags_user_name UNIQUE (user_id, lower(name))
);

-- Add index on tags table
CREATE INDEX idx_tags_user_id ON tags(user_id);

-- Junction table: Link-Category associations
CREATE TABLE link_categories (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (link_id, category_id)
);

-- Junction table: Link-Language associations (with ordering)
CREATE TABLE link_languages (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    language_id UUID NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
    order_num INTEGER NOT NULL,
    PRIMARY KEY (link_id, language_id)
);

-- Junction table: Link-License associations (with ordering)
CREATE TABLE link_licenses (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    license_id UUID NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,
    order_num INTEGER NOT NULL,
    PRIMARY KEY (link_id, license_id)
);

-- Junction table: Link-Tag associations (with ordering)
CREATE TABLE link_tags (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    order_num INTEGER NOT NULL,
    PRIMARY KEY (link_id, tag_id)
);
