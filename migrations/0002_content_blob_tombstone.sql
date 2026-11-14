-- SPDX-License-Identifier: AGPL-3.0-or-later
-- A removed content row must survive after its blob is deleted.
ALTER TABLE content DROP CONSTRAINT IF EXISTS content_hash_fkey;
