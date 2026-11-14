-- SPDX-License-Identifier: AGPL-3.0-or-later

ALTER TABLE content ADD COLUMN source_url TEXT;
ALTER TABLE content ADD COLUMN views BIGINT NOT NULL DEFAULT 0;

CREATE INDEX content_views_idx ON content (views DESC, published_at DESC);
