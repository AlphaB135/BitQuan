-- Create sample users
INSERT INTO users (id, username, email, password_hash, display_name, role, email_verified, created_at) VALUES
    ('00000000-0000-0000-0000-000000000001', 'admin', 'admin@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LeZeUfkZMBs9kYZP6', 'Administrator', 'admin', true, NOW()),
    ('00000000-0000-0000-0000-000000000002', 'john_doe', 'john@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LeZeUfkZMBs9kYZP6', 'John Doe', 'user', true, NOW()),
    ('00000000-0000-0000-0000-000000000003', 'jane_smith', 'jane@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LeZeUfkZMBs9kYZP6', 'Jane Smith', 'user', true, NOW()),
    ('00000000-0000-0000-0000-000000000004', 'bob_wilson', 'bob@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LeZeUfkZMBs9kYZP6', 'Bob Wilson', 'creator', true, NOW()),
    ('00000000-0000-0000-0000-000000000005', 'alice_johnson', 'alice@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LeZeUfkZMBs9kYZP6', 'Alice Johnson', 'creator', true, NOW())
ON CONFLICT (email) DO NOTHING;

-- Create sample prompts
INSERT INTO prompts (id, title, description, content, category, tags, price, currency, is_featured, is_active, created_by, view_count, download_count, sales_count, rating_avg, rating_count, created_at, updated_at) VALUES
    ('00000000-0000-0000-0000-000000000101', 'Business Email Generator', 'Generate professional business emails', 'You are a professional business assistant. Please draft a business email based on the following requirements:', 'business', ARRAY['email', 'business', 'professional'], 9.99, 'USD', true, true, '00000000-0000-0000-0000-000000000004', 150, 25, 10, 4.5, 20, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000102', 'Creative Writing Prompt', 'Inspire your creativity with writing prompts', 'Write a short story about: ', 'creative', ARRAY['writing', 'story', 'creative'], 4.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 200, 45, 30, 4.8, 25, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000103', 'Social Media Content', 'Create engaging social media posts', 'Create engaging social media content for: ', 'marketing', ARRAY['social', 'marketing', 'content'], 7.99, 'USD', true, true, '00000000-0000-0000-0000-000000000004', 120, 35, 15, 4.2, 18, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000104', 'Technical Documentation', 'Write technical documentation', 'Generate technical documentation for: ', 'technical', ARRAY['documentation', 'technical', 'how-to'], 14.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 80, 15, 8, 4.6, 12, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000105', 'SEO Article Writer', 'Write SEO-optimized articles', 'Write an SEO-optimized article about: ', 'seo', ARRAY['article', 'seo', 'writing'], 12.99, 'USD', true, true, '00000000-0000-0000-0000-000000000004', 300, 60, 40, 4.9, 35, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000106', 'Marketing Campaign', 'Plan a marketing campaign', 'Create a marketing campaign plan for: ', 'marketing', ARRAY['campaign', 'strategy', 'planning'], 19.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 90, 20, 12, 4.4, 16, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000107', 'Product Description', 'Write product descriptions', 'Write compelling product descriptions for: ', 'ecommerce', ARRAY['product', 'description', 'sales'], 5.99, 'USD', false, true, '00000000-0000-0000-0000-000000000004', 180, 40, 25, 4.3, 22, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000108', 'Blog Post Ideas', 'Generate blog post ideas', 'Generate blog post ideas about: ', 'blogging', ARRAY['blog', 'ideas', 'content'], 2.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 250, 55, 35, 4.7, 28, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000109', 'Customer Service Script', 'Create customer service scripts', 'Write a customer service script for: ', 'support', ARRAY['customer', 'service', 'script'], 8.99, 'USD', true, true, '00000000-0000-0000-0000-000000000004', 110, 30, 18, 4.1, 15, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000110', 'Email Newsletter', 'Create engaging email newsletters', 'Write an engaging email newsletter about: ', 'marketing', ARRAY['newsletter', 'email', 'engagement'], 6.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 160, 38, 22, 4.5, 20, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000111', 'YouTube Script', 'Write YouTube video scripts', 'Create a YouTube video script about: ', 'video', ARRAY['youtube', 'script', 'video'], 11.99, 'USD', true, true, '00000000-0000-0000-0000-000000000004', 220, 50, 30, 4.8, 24, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000112', 'Press Release', 'Write professional press releases', 'Write a professional press release for: ', 'pr', ARRAY['press', 'release', 'media'], 9.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 100, 22, 14, 4.3, 17, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000113', 'Case Study Template', 'Create case study templates', 'Create a case study template for: ', 'business', ARRAY['case', 'study', 'template'], 7.99, 'USD', false, true, '00000000-0000-0000-0000-000000000004', 140, 32, 19, 4.6, 19, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000114', 'Sales Email', 'Write effective sales emails', 'Write an effective sales email for: ', 'sales', ARRAY['sales', 'email', 'conversion'], 8.99, 'USD', true, true, '00000000-0000-0000-0000-000000000005', 190, 42, 26, 4.4, 21, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000115', 'White Paper', 'Write professional white papers', 'Create a white paper on: ', 'technical', ARRAY['white', 'paper', 'research'], 24.99, 'USD', false, true, '00000000-0000-0000-0000-000000000004', 70, 18, 9, 4.7, 13, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000116', 'Ad Copy', 'Create compelling ad copy', 'Write compelling ad copy for: ', 'advertising', ARRAY['ad', 'copy', 'marketing'], 5.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 170, 36, 21, 4.2, 19, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000117', 'Landing Page', 'Write landing page content', 'Create landing page content for: ', 'marketing', ARRAY['landing', 'page', 'conversion'], 10.99, 'USD', true, true, '00000000-0000-0000-0000-000000000004', 210, 48, 28, 4.6, 23, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000118', 'Tutorial Guide', 'Create step-by-step tutorials', 'Write a step-by-step tutorial for: ', 'education', ARRAY['tutorial', 'guide', 'learn'], 4.99, 'USD', false, true, '00000000-0000-0000-0000-000000000005', 200, 45, 30, 4.8, 25, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000119', 'Company Bio', 'Write company bios', 'Write a compelling company bio for: ', 'business', ARRAY['company', 'bio', 'about'], 6.99, 'USD', false, true, '00000000-0000-0000-0000-000000000004', 130, 28, 16, 4.3, 16, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000120', 'Job Description', 'Write job descriptions', 'Create a job description for: ', 'hr', ARRAY['job', 'description', 'hiring'], 3.99, 'USD', true, true, '00000000-0000-0000-0000-000000000005', 160, 35, 20, 4.5, 20, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- Create sample ratings
INSERT INTO ratings (id, prompt_id, user_id, rating, comment, created_at) VALUES
    ('00000000-0000-0000-0000-000000000201', '00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000002', 5, 'Excellent email generator!', NOW()),
    ('00000000-0000-0000-0000-000000000202', '00000000-0000-0000-0000-000000000102', '00000000-0000-0000-0000-000000000003', 4, 'Very creative prompts', NOW()),
    ('00000000-0000-0000-0000-000000000203', '00000000-0000-0000-0000-000000000103', '00000000-0000-0000-0000-000000000002', 5, 'Great social media content', NOW()),
    ('00000000-0000-0000-0000-000000000204', '00000000-0000-0000-0000-000000000104', '00000000-0000-0000-0000-000000000003', 4, 'Good technical documentation', NOW()),
    ('00000000-0000-0000-0000-000000000205', '00000000-0000-0000-0000-000000000105', '00000000-0000-0000-0000-000000000002', 5, 'Amazing SEO writer', NOW())
ON CONFLICT DO NOTHING;

-- Create sample orders
INSERT INTO orders (id, user_id, prompt_id, amount, currency, status, payment_method, transaction_id, created_at, updated_at) VALUES
    ('00000000-0000-0000-0000-000000000301', '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000101', 9.99, 'USD', 'completed', 'credit_card', 'txn_123456789', NOW() - interval '1 day', NOW() - interval '1 day'),
    ('00000000-0000-0000-0000-000000000302', '00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000102', 4.99, 'USD', 'completed', 'paypal', 'txn_234567890', NOW() - interval '2 days', NOW() - interval '2 days'),
    ('00000000-0000-0000-0000-000000000303', '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000103', 7.99, 'USD', 'completed', 'credit_card', 'txn_345678901', NOW() - interval '3 days', NOW() - interval '3 days'),
    ('00000000-0000-0000-0000-000000000304', '00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000104', 14.99, 'USD', 'completed', 'stripe', 'txn_456789012', NOW() - interval '4 days', NOW() - interval '4 days'),
    ('00000000-0000-0000-0000-000000000305', '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000105', 12.99, 'USD', 'completed', 'paypal', 'txn_567890123', NOW() - interval '5 days', NOW() - interval '5 days'),
    ('00000000-0000-0000-0000-000000000306', '00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000106', 19.99, 'USD', 'pending', 'credit_card', 'txn_678901234', NOW() - interval '1 hour', NOW() - interval '30 minutes'),
    ('00000000-0000-0000-0000-000000000307', '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000107', 5.99, 'USD', 'completed', 'stripe', 'txn_789012345', NOW() - interval '6 days', NOW() - interval '6 days'),
    ('00000000-0000-0000-0000-000000000308', '00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000108', 2.99, 'USD', 'completed', 'paypal', 'txn_890123456', NOW() - interval '7 days', NOW() - interval '7 days')
ON CONFLICT (id) DO NOTHING;

-- Update sales counts for prompts
UPDATE prompts SET sales_count = sales_count + 1 WHERE id IN (
    '00000000-0000-0000-0000-000000000101',
    '00000000-0000-0000-0000-000000000102',
    '00000000-0000-0000-0000-000000000103',
    '00000000-0000-0000-0000-000000000104',
    '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000107',
    '00000000-0000-0000-0000-000000000108'
);
