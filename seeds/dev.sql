-- Development/demo account used by the web and WeChat clients.
-- Remove this file from the compose volume mounts for a production install.
INSERT INTO users (id, email, provider, provider_id, display_name)
VALUES (
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11',
    'demo@example.com',
    'dev',
    'demo',
    'Demo User'
)
ON CONFLICT DO NOTHING;
