-- Seed data for predefined languages and licenses
-- These are global defaults available to all users (user_id = NULL)
-- Individual users can add their own custom languages and licenses

-- Insert predefined programming languages
INSERT INTO languages (id, user_id, name) VALUES
    (gen_random_uuid(), NULL, 'JavaScript'),
    (gen_random_uuid(), NULL, 'Python'),
    (gen_random_uuid(), NULL, 'Java'),
    (gen_random_uuid(), NULL, 'C#'),
    (gen_random_uuid(), NULL, 'C++'),
    (gen_random_uuid(), NULL, 'TypeScript'),
    (gen_random_uuid(), NULL, 'PHP'),
    (gen_random_uuid(), NULL, 'C'),
    (gen_random_uuid(), NULL, 'Ruby'),
    (gen_random_uuid(), NULL, 'Go'),
    (gen_random_uuid(), NULL, 'Rust'),
    (gen_random_uuid(), NULL, 'Swift'),
    (gen_random_uuid(), NULL, 'Kotlin'),
    (gen_random_uuid(), NULL, 'R'),
    (gen_random_uuid(), NULL, 'Dart'),
    (gen_random_uuid(), NULL, 'Scala'),
    (gen_random_uuid(), NULL, 'Perl'),
    (gen_random_uuid(), NULL, 'Lua'),
    (gen_random_uuid(), NULL, 'Haskell'),
    (gen_random_uuid(), NULL, 'Elixir');

-- Insert predefined software licenses with acronyms and full names
INSERT INTO licenses (id, user_id, name, full_name) VALUES
    (gen_random_uuid(), NULL, 'MIT', 'MIT License'),
    (gen_random_uuid(), NULL, 'Apache-2.0', 'Apache License 2.0'),
    (gen_random_uuid(), NULL, 'GPL-3.0', 'GNU General Public License v3.0'),
    (gen_random_uuid(), NULL, 'GPL-2.0', 'GNU General Public License v2.0'),
    (gen_random_uuid(), NULL, 'BSD-3-Clause', 'BSD 3-Clause "New" or "Revised" License'),
    (gen_random_uuid(), NULL, 'BSD-2-Clause', 'BSD 2-Clause "Simplified" License'),
    (gen_random_uuid(), NULL, 'LGPL-3.0', 'GNU Lesser General Public License v3.0'),
    (gen_random_uuid(), NULL, 'LGPL-2.1', 'GNU Lesser General Public License v2.1'),
    (gen_random_uuid(), NULL, 'MPL-2.0', 'Mozilla Public License 2.0'),
    (gen_random_uuid(), NULL, 'AGPL-3.0', 'GNU Affero General Public License v3.0'),
    (gen_random_uuid(), NULL, 'ISC', 'ISC License'),
    (gen_random_uuid(), NULL, 'CDDL-1.0', 'Common Development and Distribution License 1.0'),
    (gen_random_uuid(), NULL, 'EPL-2.0', 'Eclipse Public License 2.0'),
    (gen_random_uuid(), NULL, 'EPL-1.0', 'Eclipse Public License 1.0'),
    (gen_random_uuid(), NULL, 'CC0-1.0', 'Creative Commons Zero v1.0 Universal'),
    (gen_random_uuid(), NULL, 'CC-BY-4.0', 'Creative Commons Attribution 4.0'),
    (gen_random_uuid(), NULL, 'CC-BY-SA-4.0', 'Creative Commons Attribution Share Alike 4.0'),
    (gen_random_uuid(), NULL, 'Unlicense', 'The Unlicense'),
    (gen_random_uuid(), NULL, 'Zlib', 'zlib License'),
    (gen_random_uuid(), NULL, 'Artistic-2.0', 'Artistic License 2.0');
