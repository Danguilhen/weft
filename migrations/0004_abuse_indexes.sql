-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE INDEX IF NOT EXISTS uploads_client_created_idx
    ON uploads (client_hash, created_at DESC);
CREATE INDEX IF NOT EXISTS content_status_created_idx
    ON content (status, created_at);
CREATE INDEX IF NOT EXISTS reports_created_idx
    ON reports (created_at);
