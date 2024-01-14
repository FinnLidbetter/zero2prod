CREATE TABLE newsletter_issues (
    newsletter_issue_id uuid NOT NULL,
    title TEXT NOT NUll,
    text_content TEXT NOT NULL,
    html_content TEXT NOT NULL,
    published_at timestamptz NULL,
    PRIMARY KEY(newsletter_issue_id)
);