# Publishing Automation Plan

**Date:** 2026-03-26
**Project:** YonedaAI Social Media Automation
**Post inventory:** 21 ready-to-post files in `posts/social/`

---

## Platform API Matrix

| Platform | API Available | Auth Method | Library | Cost | Automation Level | Notes |
|----------|:---:|-------------|---------|------|:---:|-------|
| Twitter/X | Yes | OAuth 2.0 | `twitter-api-v2` (npm) | Free: $0 (500 writes/mo) / Basic: $200/mo | Full | Basic tier doubled to $200/mo in 2025; new pay-per-use option in beta (Feb 2026) |
| LinkedIn | Yes | OAuth 2.0 | Direct REST / `linkedin-api` | Free | Full | Must request "Share on LinkedIn" permission; no scheduled posts via API; 3-legged OAuth required |
| Mastodon | Yes | OAuth 2.0 / Access Token | `masto` (npm) / `Mastodon.py` | Free | Full | Best API of all platforms; fully open; PKCE supported on 4.3+ |
| Bluesky | Yes | App Password | `@atproto/api` (npm) / `atproto` (Python) | Free | Full | No built-in scheduling; excellent SDK support |
| Dev.to | Yes | API Key | Direct REST | Free | Full | Simple REST API; create/update/publish articles; API key from Settings |
| Medium | Deprecated | Integration Token | Direct REST | Free | Partial | API officially unsupported; no new tokens issued; existing tokens still work; can create drafts only |
| Reddit | Yes | OAuth 2.0 | `snoowrap` (npm) / `praw` (Python) | Free tier for non-commercial | Semi-auto | $0.24/1K calls commercial; strict anti-spam; automated posting will get banned; prepare-then-manual-submit recommended |
| Facebook | Yes (Pages only) | Page Access Token | Direct REST (Graph API) | Free | Full (Pages) | Business app required; personal profiles not supported; app review may be required for non-page-admin use |
| Hacker News | Read-only | None | N/A | Free | Manual | No write API exists. Period. Manual submission only |
| Lobsters | No | N/A | N/A | Free | Manual | Invite-only; no API for posting |
| Tildes | No | N/A | N/A | Free | Manual | No API at all |
| Lambda the Ultimate | No | N/A | N/A | Free | Manual | Drupal-based; no API |
| Stack Exchange | Yes (limited) | OAuth 2.0 | Direct REST | Free | Semi-auto | Write access heavily restricted to prevent spam; manual recommended |
| Discord | Yes (Bot) | Bot Token | `discord.js` | Free | Full (if bot) | Need bot in each server; requires server admin approval |
| Zulip | Yes | API Key | `zulip-js` | Free | Full | Good API; need account on each instance |
| Discourse Forums | Yes | API Key | Direct REST | Free | Full (if admin/user key) | Per-instance; User API Keys available without admin access |

---

## Honest Assessment: What Can Actually Be Automated

### Fully Automatable (API + library + no friction)
1. **Mastodon** -- best API, zero cost, zero gatekeeping
2. **Bluesky** -- excellent API, app password auth, straightforward
3. **Dev.to** -- simple REST API, API key from settings, publish directly
4. **Twitter/X** -- API works well but costs $200/mo for Basic tier; free tier allows 500 writes/mo (sufficient for our 21 posts)

### Automatable With Effort (requires app setup, approval, or OAuth dance)
5. **LinkedIn** -- requires Developer App + OAuth 2.0 consent flow + "Share on LinkedIn" permission request
6. **Facebook Pages** -- requires Business app + Page Access Token; app review process is slow and opaque
7. **Discourse Forums** (Rust, Elixir, OCaml) -- need User API Key per forum instance; some forums may not allow it

### Semi-Automatable (prepare content, submit manually)
8. **Reddit** -- prepare posts in script, open browser with pre-filled content; automated posting WILL get flagged
9. **Stack Exchange** -- same approach; API write access is restricted

### Manual Only (no write API)
10. **Hacker News** -- manual, no workaround
11. **Lobsters** -- manual + need invite first
12. **Tildes** -- manual
13. **Lambda the Ultimate** -- manual
14. **Discord** -- manual unless you run a bot in each server (impractical for community servers)
15. **Zulip** -- manual for communities where you lack API access
16. **Meetups/Hackerspaces** -- in-person or email

---

## Recommended Stack: Option C (Hybrid)

After researching all options, the hybrid approach is the clear winner.

### Why Not a Unified Service (Buffer/Publer)?

| Service | Platforms Covered | Missing | Cost | API for Automation? |
|---------|------------------|---------|------|:---:|
| **Publer** | Twitter, LinkedIn, Facebook, Mastodon, Bluesky + 8 more | Dev.to, Medium, Reddit, HN | $12/mo (Professional) | Yes |
| **Buffer** | Twitter, LinkedIn, Facebook, Mastodon, Bluesky, Threads | Dev.to, Medium, Reddit, HN | $6/channel/mo | Yes |

**Verdict:** Publer or Buffer can handle Tier 1 social platforms (Twitter, LinkedIn, Facebook, Mastodon, Bluesky) through a single UI with visual scheduling. This saves building OAuth flows and token management for 5 platforms. But you still need custom scripts for Dev.to, Medium, and semi-auto workflows for Reddit.

### Recommended Architecture

```
+------------------------------------------+
|           Publishing Controller           |
|         scripts/publish.ts                |
+------------------------------------------+
         |              |              |
    +----v----+   +----v----+   +----v----+
    | Publer  |   | Custom  |   | Manual  |
    | (or     |   | REST    |   | Checklist|
    | Buffer) |   | Scripts |   | (CLI)   |
    +---------+   +---------+   +---------+
    | Twitter |   | Dev.to  |   | HN      |
    | LinkedIn|   | Medium  |   | Lobsters|
    | Facebook|   | Bluesky*|   | Tildes  |
    | Mastodon|   | Reddit  |   | LtU     |
    |         |   | (prep)  |   | Discord |
    +---------+   +---------+   +---------+

    * Bluesky can go in either Publer or Custom
```

**Decision point:** If you want zero monthly cost, skip Publer/Buffer and write custom adapters for everything. If you value a scheduling UI and want to save 8+ hours of OAuth implementation, use Publer ($12/mo).

---

## API Keys and Accounts Needed

### Accounts to Create (in order of priority)

| # | Platform | Where to Register | What You Get | Time |
|---|----------|------------------|-------------|------|
| 1 | Twitter/X | [developer.x.com](https://developer.x.com) | API Key, API Secret, Bearer Token, Access Token + Secret | 15 min |
| 2 | Bluesky | Settings > App Passwords | App Password | 2 min |
| 3 | Mastodon | Instance Settings > Development > New Application | Access Token | 5 min |
| 4 | Dev.to | Settings > Extensions > DEV API Keys | API Key | 2 min |
| 5 | LinkedIn | [developer.linkedin.com](https://developer.linkedin.com) > Create App | Client ID + Client Secret | 20 min |
| 6 | Medium | Settings > Security and apps > Integration tokens | Integration Token (if available; may not issue new ones) | 5 min |
| 7 | Reddit | [reddit.com/prefs/apps](https://www.reddit.com/prefs/apps) > Create App (script type) | Client ID + Client Secret | 10 min |
| 8 | Facebook | [developers.facebook.com](https://developers.facebook.com) > Create App (Business type) | Page Access Token | 30-60 min |
| 9 | Publer (optional) | [publer.io](https://publer.io) | API access on Professional plan | 10 min |

**Total setup time:** ~1.5-2 hours for all accounts

---

## .env Template

```bash
# =============================================================================
# YonedaAI Publishing Automation - Environment Variables
# =============================================================================
# NEVER commit this file. Add to .gitignore immediately.

# --- Twitter/X ---
# Free tier: 500 writes/month (enough for our 21 posts)
# Basic ($200/mo): 50K writes/month + read access
TWITTER_API_KEY=
TWITTER_API_SECRET=
TWITTER_BEARER_TOKEN=
TWITTER_ACCESS_TOKEN=
TWITTER_ACCESS_SECRET=

# --- LinkedIn ---
# Requires OAuth 2.0 consent flow; access token expires in 60 days
LINKEDIN_CLIENT_ID=
LINKEDIN_CLIENT_SECRET=
LINKEDIN_ACCESS_TOKEN=

# --- Mastodon ---
# Get from: Instance Settings > Development > New Application
# Scopes needed: read write:statuses write:media
MASTODON_INSTANCE=https://mastodon.social
MASTODON_ACCESS_TOKEN=

# --- Bluesky ---
# Get from: Settings > App Passwords
BLUESKY_HANDLE=
BLUESKY_APP_PASSWORD=

# --- Medium ---
# WARNING: Medium no longer issues new integration tokens.
# If you have an existing token, it still works.
MEDIUM_INTEGRATION_TOKEN=

# --- Dev.to ---
# Get from: Settings > Extensions > DEV API Keys
DEVTO_API_KEY=

# --- Reddit ---
# Create "script" type app at reddit.com/prefs/apps
# WARNING: Automated posting is risky. Use for content prep only.
REDDIT_CLIENT_ID=
REDDIT_CLIENT_SECRET=
REDDIT_USERNAME=
REDDIT_PASSWORD=

# --- Facebook (Page) ---
# Requires Business app + Page admin access
# Page tokens don't expire if generated via long-lived token flow
FACEBOOK_PAGE_ID=
FACEBOOK_PAGE_ACCESS_TOKEN=

# --- Publer (optional) ---
PUBLER_API_KEY=

# --- Discourse Forums (per-instance) ---
DISCOURSE_RUST_API_KEY=
DISCOURSE_RUST_USERNAME=
DISCOURSE_ELIXIR_API_KEY=
DISCOURSE_ELIXIR_USERNAME=
DISCOURSE_OCAML_API_KEY=
DISCOURSE_OCAML_USERNAME=
```

---

## Implementation Plan

### Phase 1: Immediate Setup (Day 1, ~2 hours)

**1a. Create accounts and get API keys** (see table above)

**1b. Add `.env` to `.gitignore`**
```bash
echo ".env" >> .gitignore
echo ".publish-state.json" >> .gitignore
```

**1c. Decide: Publer or custom-only?**
- If Publer: sign up, connect Twitter/LinkedIn/Facebook/Mastodon/Bluesky accounts, upload first batch via their UI
- If custom-only: proceed to Phase 2

### Phase 2: Build Automation Script (Day 1-2, ~4-6 hours)

#### Script Architecture

```
scripts/
  publish.ts              # Main entry point / CLI
  lib/
    adapters/
      twitter.ts          # Twitter/X via twitter-api-v2
      mastodon.ts         # Mastodon via masto
      bluesky.ts          # Bluesky via @atproto/api
      devto.ts            # Dev.to via REST
      medium.ts           # Medium via REST (draft only)
      linkedin.ts         # LinkedIn via REST + OAuth
      facebook.ts         # Facebook Graph API
      reddit.ts           # Reddit content prep (opens browser)
      discourse.ts        # Discourse forums via REST
    parser.ts             # Parse post files from posts/social/
    state.ts              # Track what's been posted
    scheduler.ts          # Timing and rate limiting
    types.ts              # Shared types
  .publish-state.json     # Tracks posted status per platform
```

#### Core Types

```typescript
// scripts/lib/types.ts

interface PlatformAdapter {
  name: string;
  isConfigured(): boolean;
  post(content: PostContent): Promise<PostResult>;
  postThread?(parts: string[]): Promise<PostResult>;
}

interface PostContent {
  title?: string;         // For articles (Dev.to, Medium)
  body: string;           // Main content
  tags?: string[];        // Platform tags/hashtags
  url?: string;           // Link to include
  images?: string[];      // Image paths
  canonical_url?: string; // For cross-posted articles
}

interface PostResult {
  success: boolean;
  platform: string;
  url?: string;           // URL of published post
  id?: string;            // Platform-specific post ID
  error?: string;
  draftOnly?: boolean;    // True for Medium
}

interface PublishState {
  posts: {
    [filename: string]: {
      [platform: string]: {
        posted: boolean;
        url?: string;
        timestamp?: string;
        error?: string;
      };
    };
  };
}

interface PublishOptions {
  file?: string;          // Specific file to post
  platform?: string;      // Specific platform (or "all")
  dryRun?: boolean;       // Print what would be posted
  schedule?: string;      // ISO date for scheduling
}
```

#### CLI Interface

```bash
# Post a specific file to a specific platform
npx tsx scripts/publish.ts --file twitter-japl.md --platform twitter

# Dry run (show what would be posted, don't actually post)
npx tsx scripts/publish.ts --file twitter-japl.md --platform twitter --dry-run

# Post to all configured platforms
npx tsx scripts/publish.ts --file mastodon-posts.md --platform all

# Show status of all posts
npx tsx scripts/publish.ts --status

# Post everything that hasn't been posted yet to a platform
npx tsx scripts/publish.ts --platform devto --all-pending
```

#### Dependencies

```json
{
  "devDependencies": {
    "tsx": "^4.0.0",
    "typescript": "^5.4.0"
  },
  "dependencies": {
    "twitter-api-v2": "^1.17.0",
    "masto": "^6.8.0",
    "@atproto/api": "^0.13.0",
    "dotenv": "^16.4.0",
    "commander": "^12.0.0",
    "open": "^10.0.0"
  }
}
```

### Phase 3: Platform Adapter Implementation Details

#### Twitter/X Adapter
```typescript
// Key points:
// - Free tier: 500 writes/month, NO read access
// - twitter-api-v2 handles OAuth 2.0 and v2 endpoints
// - Thread posting: post first tweet, then reply to it in sequence
// - Media upload: use v1.1 media/upload endpoint (still required)
// - Rate limit: 1 request per second for posting

import { TwitterApi } from 'twitter-api-v2';

const client = new TwitterApi({
  appKey: process.env.TWITTER_API_KEY!,
  appSecret: process.env.TWITTER_API_SECRET!,
  accessToken: process.env.TWITTER_ACCESS_TOKEN!,
  accessSecret: process.env.TWITTER_ACCESS_SECRET!,
});

// Single tweet
await client.v2.tweet({ text: content });

// Thread (reply chain)
let lastTweet = await client.v2.tweet({ text: tweets[0] });
for (const tweet of tweets.slice(1)) {
  lastTweet = await client.v2.reply(tweet, lastTweet.data.id);
}
```

#### Mastodon Adapter
```typescript
// Key points:
// - Best API of all platforms, fully featured
// - Access token from instance settings (no OAuth dance needed)
// - Supports: posts, threads (reply chains), media, polls, scheduled posts
// - Character limit: 500 (default, some instances allow more)
// - Rate limit: 300 requests per 5 minutes

import { createRestAPIClient } from 'masto';

const masto = createRestAPIClient({
  url: process.env.MASTODON_INSTANCE!,
  accessToken: process.env.MASTODON_ACCESS_TOKEN!,
});

// Single post
await masto.v1.statuses.create({ status: content, visibility: 'public' });

// Thread
let lastStatus = await masto.v1.statuses.create({ status: parts[0] });
for (const part of parts.slice(1)) {
  lastStatus = await masto.v1.statuses.create({
    status: part,
    inReplyToId: lastStatus.id,
  });
}
```

#### Bluesky Adapter
```typescript
// Key points:
// - AT Protocol, fully open
// - Auth via identifier (handle) + app password
// - Rich text with facets for links/mentions/hashtags
// - No built-in scheduling
// - 300 char limit per post (grapheme-based)

import { BskyAgent, RichText } from '@atproto/api';

const agent = new BskyAgent({ service: 'https://bsky.social' });
await agent.login({
  identifier: process.env.BLUESKY_HANDLE!,
  password: process.env.BLUESKY_APP_PASSWORD!,
});

const rt = new RichText({ text: content });
await rt.detectFacets(agent);
await agent.post({
  text: rt.text,
  facets: rt.facets,
});
```

#### Dev.to Adapter
```typescript
// Key points:
// - Simplest API of the bunch
// - API key in header, POST JSON body
// - Can publish directly (published: true) or as draft
// - Supports markdown body, tags, canonical_url, cover image
// - Rate limit: 30 requests per 30 seconds

await fetch('https://dev.to/api/articles', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'api-key': process.env.DEVTO_API_KEY!,
  },
  body: JSON.stringify({
    article: {
      title: 'JAPL: A Self-Hosting Language...',
      body_markdown: markdownContent,
      published: false, // Start as draft, review, then publish
      tags: ['programming-languages', 'compilers', 'typescript'],
      canonical_url: 'https://japl-nine.vercel.app',
    },
  }),
});
```

#### Medium Adapter
```typescript
// Key points:
// - API is officially deprecated but still functional
// - No new integration tokens being issued (check if yours works)
// - Can only create DRAFTS, not publish directly
// - Supports markdown and HTML content
// - Must get authorId first via GET /v1/me

const userId = await fetch('https://api.medium.com/v1/me', {
  headers: { Authorization: `Bearer ${process.env.MEDIUM_INTEGRATION_TOKEN}` },
}).then(r => r.json()).then(d => d.data.id);

await fetch(`https://api.medium.com/v1/users/${userId}/posts`, {
  method: 'POST',
  headers: {
    Authorization: `Bearer ${process.env.MEDIUM_INTEGRATION_TOKEN}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    title: 'Article Title',
    contentFormat: 'markdown',
    content: markdownContent,
    publishStatus: 'draft', // Only option that works reliably
    tags: ['programming', 'type-theory'],
    canonicalUrl: 'https://japl-nine.vercel.app',
  }),
});
```

#### LinkedIn Adapter
```typescript
// Key points:
// - Requires OAuth 2.0 three-legged flow (browser redirect)
// - Access token expires in 60 days; refresh tokens last 365 days
// - Must request "Share on LinkedIn" product access
// - Posts use the "ugcPosts" or "posts" endpoint
// - No scheduling via API; immediate publish only
// - Media requires multi-step upload (register, upload, reference)

// First: run OAuth flow once to get access token (interactive, browser-based)
// Then: use token for posting

await fetch('https://api.linkedin.com/v2/ugcPosts', {
  method: 'POST',
  headers: {
    Authorization: `Bearer ${process.env.LINKEDIN_ACCESS_TOKEN}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    author: `urn:li:person:${personId}`,
    lifecycleState: 'PUBLISHED',
    specificContent: {
      'com.linkedin.ugc.ShareContent': {
        shareCommentary: { text: postContent },
        shareMediaCategory: 'ARTICLE',
        media: [{
          status: 'READY',
          originalUrl: 'https://japl-nine.vercel.app',
        }],
      },
    },
    visibility: { 'com.linkedin.ugc.MemberNetworkVisibility': 'PUBLIC' },
  }),
});
```

#### Reddit Adapter (Prep-Only)
```typescript
// Key points:
// - DO NOT fully automate Reddit posting. You WILL get banned.
// - Instead: prepare content and open browser with pre-filled URL
// - Reddit's submit page supports URL params for pre-filling
// - Review each post manually before submitting
// - Space submissions across days and subreddits

import open from 'open';

function prepareRedditPost(subreddit: string, title: string, url: string) {
  const submitUrl = new URL(`https://www.reddit.com/r/${subreddit}/submit`);
  submitUrl.searchParams.set('type', 'link');  // or 'self' for text
  submitUrl.searchParams.set('title', title);
  submitUrl.searchParams.set('url', url);

  console.log(`Opening Reddit submission for r/${subreddit}...`);
  open(submitUrl.toString());
}
```

### Phase 4: Post File Parser

The parser needs to handle the different formats in `posts/social/`:

```typescript
// scripts/lib/parser.ts

interface ParsedPost {
  platform: string;          // twitter, linkedin, mastodon, devto, etc.
  project: string;           // japl, mra, yoneda-constraint
  format: 'thread' | 'article' | 'single' | 'multi';
  parts: string[];           // Individual tweets/posts (for threads)
  title?: string;            // For articles (Dev.to, Medium)
  tags?: string[];           // Hashtags/tags
  frontmatter?: Record<string, any>;  // YAML frontmatter (Dev.to)
}

// File naming convention in posts/social/:
//   twitter-japl.md        -> platform=twitter, project=japl
//   devto-japl.md          -> platform=devto, project=japl
//   mastodon-posts.md      -> platform=mastodon, project=multi
//   hn-submissions.md      -> platform=hn, project=multi
//
// Twitter files: split on "## Tweet N" headings
// Dev.to files: parse YAML frontmatter + markdown body
// Mastodon files: split on "## Post N" headings
// LinkedIn files: single post body
// Medium files: full article markdown
```

### Phase 5: State Tracking

```typescript
// scripts/lib/state.ts
// Tracks what has been posted where, stored in .publish-state.json

// Example state:
{
  "posts": {
    "twitter-japl.md": {
      "twitter": {
        "posted": true,
        "url": "https://twitter.com/YonedaAI/status/123456",
        "timestamp": "2026-03-27T14:30:00Z"
      }
    },
    "devto-japl.md": {
      "devto": {
        "posted": true,
        "url": "https://dev.to/yonedaai/japl-abc123",
        "timestamp": "2026-03-28T10:00:00Z"
      }
    },
    "mastodon-posts.md": {
      "mastodon": {
        "posted": false,
        "error": "Rate limited, retry after 2026-03-28T15:00:00Z"
      }
    }
  }
}
```

### Phase 6: Scheduling (Optional)

#### Option A: GitHub Actions Cron

```yaml
# .github/workflows/publish.yml
name: Scheduled Publishing
on:
  schedule:
    - cron: '0 14 * * 1-5'  # 2 PM UTC (9 AM EST) weekdays
  workflow_dispatch:
    inputs:
      platform:
        description: 'Platform to post to'
        required: true
        type: choice
        options: [twitter, mastodon, bluesky, devto, linkedin, all]
      dry_run:
        description: 'Dry run?'
        required: false
        type: boolean
        default: true

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npx tsx scripts/publish.ts --platform ${{ inputs.platform || 'all' }} --all-pending ${{ inputs.dry_run && '--dry-run' || '' }}
        env:
          TWITTER_API_KEY: ${{ secrets.TWITTER_API_KEY }}
          TWITTER_API_SECRET: ${{ secrets.TWITTER_API_SECRET }}
          # ... all other secrets
```

#### Option B: Publer Scheduling UI
- Upload content via Publer's bulk scheduling
- Use their calendar view for visual scheduling
- Covers Twitter, LinkedIn, Facebook, Mastodon, Bluesky
- $12/month for Professional plan

#### Option C: Local cron (simplest)
```bash
# Post one pending item per day at 9 AM EST
0 14 * * 1-5 cd /path/to/japl && npx tsx scripts/publish.ts --platform all --next-pending
```

---

## Cost Estimate

| Item | Monthly Cost | Annual Cost | Notes |
|------|-------------|-------------|-------|
| Twitter/X API (Free) | $0 | $0 | 500 writes/month; sufficient for 21 posts + future content |
| Twitter/X API (Basic) | $200 | $2,400 | Only if you need read access or >500 writes/month |
| Publer Professional | $12 | $144 | Optional; for scheduling UI across 5+ platforms |
| Buffer Essentials | $6/channel | $72/channel | Alternative to Publer |
| LinkedIn | $0 | $0 | Free developer app |
| Mastodon | $0 | $0 | Free, open API |
| Bluesky | $0 | $0 | Free, open API |
| Dev.to | $0 | $0 | Free API |
| Medium | $0 | $0 | Free (if you have existing token) |
| Reddit | $0 | $0 | Free for non-commercial |
| Facebook | $0 | $0 | Free (requires business app setup) |
| Discourse | $0 | $0 | Free per-instance API |
| **Total (minimal)** | **$0** | **$0** | Custom scripts only, Twitter free tier |
| **Total (recommended)** | **$12** | **$144** | Add Publer for scheduling UI |
| **Total (with Twitter Basic)** | **$212** | **$2,544** | If read access needed |

---

## Manual Posting Checklist

For platforms without write APIs or where automation is risky.

### Hacker News (news.ycombinator.com)
- [ ] Go to [https://news.ycombinator.com/submit](https://news.ycombinator.com/submit)
- [ ] **Submission 1:** Title: "The Yoneda Constraint: One categorical axiom unifying Godel, the measurement problem, and AI alignment" / URL: https://yoneda-constraint.vercel.app
- [ ] **Submission 2:** Title: "Show HN: JAPL -- a self-hosting FP language that compiles to TypeScript and C" / URL: https://japl-nine.vercel.app
- [ ] **Submission 3:** Title: "The Minimal Runtime Axiom: Every runtime decision is a theorem you didn't prove" / URL: https://minimal-runtime-axiom.vercel.app
- [ ] Best time: Tuesday-Thursday, 8-10 AM EST
- [ ] Space submissions 3-5 days apart

### Lobsters (lobste.rs)
- [ ] Secure an invite first (ask in relevant communities or from existing members)
- [ ] Submit JAPL and Yoneda Constraint (see `posts/social/lobsters-submissions.md`)
- [ ] Add relevant tags when submitting

### Tildes (tildes.net)
- [ ] Submit to `~comp` (JAPL, MRA) and `~science` (Yoneda Constraint)
- [ ] Write substantial introductory comments (Tildes values discussion)
- [ ] See `posts/social/tildes-submissions.md`

### Lambda the Ultimate (lambda-the-ultimate.org)
- [ ] Create account if needed
- [ ] Submit as discussion posts with rigorous introductions
- [ ] See `posts/social/lambda-the-ultimate.md`
- [ ] LtU audience expects formal rigor; proofread carefully

### Reddit (manual submission recommended)
- [ ] Use the automation script's `--platform reddit` to open pre-filled submission pages
- [ ] Review each post before submitting
- [ ] Space across subreddits (1-2 per day maximum)
- [ ] Engage in comments after posting (don't post-and-ghost)
- [ ] See `posts/social/` for subreddit-specific content

### Stack Exchange
- [ ] CS Theory: Frame as genuine questions inviting discussion
- [ ] Stack Overflow: Self-answered Q&A format
- [ ] LangDev: Design discussion format
- [ ] See `posts/social/stackexchange-posts.md`

### Discord Servers
- [ ] Join relevant servers first (PL Design, Type Theory, FP, Category Theory)
- [ ] Introduce yourself before posting links
- [ ] Share in appropriate channels (not general/off-topic)
- [ ] See `posts/social/discord-zulip-posts.md` and `posts/social/gleam-discord.md`

### Zulip Communities (Lean, Agda)
- [ ] Create accounts on relevant Zulip instances
- [ ] Post in appropriate streams/topics
- [ ] See `posts/social/discord-zulip-posts.md`

### Discourse Forums
- [ ] **Rust Users** (discuss.rust-lang.org): Post about ownership model inspiration
- [ ] **Elixir Forum** (elixirforum.com): Post about Erlang-inspired process model
- [ ] **OCaml Discuss**: Post about ML-family type system connections
- [ ] See `posts/social/discourse-forums.md`
- [ ] Can be automated if API keys are obtained (see Discourse adapter above)

### Meetups / Hackerspaces
- [ ] Papers We Love Chicago: nominate Yoneda Constraint
- [ ] Chicago FP/Haskell meetup: propose JAPL talk
- [ ] Local hackerspaces: lightning talk on self-hosting compilers
- [ ] Recurse Center community: share in appropriate channels
- [ ] See `posts/social/meetups-hackerspaces.md`

---

## Execution Timeline

### Day 1: Setup
| Time | Task |
|------|------|
| 0:00-0:30 | Create Twitter Developer account, get API keys |
| 0:30-0:45 | Create Bluesky app password, Mastodon application token, Dev.to API key |
| 0:45-1:15 | Create LinkedIn Developer App, start OAuth flow |
| 1:15-1:30 | Check Medium integration token availability |
| 1:30-2:00 | Create Reddit app, Facebook developer app (start process) |
| 2:00-2:30 | Set up `.env` file with all keys |
| 2:30-4:00 | Build core publish.ts script + Mastodon adapter (easiest, for testing) |

### Day 2: Build Adapters
| Time | Task |
|------|------|
| 0:00-1:00 | Twitter adapter + thread support |
| 1:00-1:30 | Bluesky adapter |
| 1:30-2:00 | Dev.to adapter |
| 2:00-2:30 | Medium adapter (draft only) |
| 2:30-3:00 | Reddit prep adapter (browser opener) |
| 3:00-3:30 | Post file parser for all 21 files |
| 3:30-4:00 | State tracking + dry-run mode |

### Day 3: Post Wave 1
| Time | Task |
|------|------|
| Morning | Post Yoneda Constraint to: Twitter, Mastodon, Bluesky, LinkedIn |
| Morning | Submit to HN manually |
| Afternoon | Post to Reddit (r/math, r/PhilosophyofScience) manually |
| Evening | Monitor engagement, respond to comments |

### Days 4-14: Follow Publishing Schedule
- Follow the 14-day posting sequence from PUBLISHING.md
- Use automation script for API platforms
- Manual checklist for community platforms
- Track everything in `.publish-state.json`

---

## Key Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Twitter free tier rate limit hit | Low | Low | 500 writes/month is plenty for 21 posts + threads |
| LinkedIn OAuth token expires | High (60 days) | Medium | Implement token refresh flow; set calendar reminder |
| Medium token unavailable | Medium | Low | Medium is a nice-to-have; cross-post manually if needed |
| Reddit account flagged for spam | High if automated | High | NEVER fully automate; always manual review + submit |
| Facebook app review rejected | Medium | Low | Facebook is lowest priority; use personal sharing if needed |
| HN submission doesn't gain traction | Medium | Medium | Optimal timing (Tue-Thu 9AM EST); compelling titles; engage in comments |
| API breaking changes | Low (short term) | Medium | Pin library versions; monitor changelogs |

---

## Summary of Recommendations

1. **Start with the free platforms that have the best APIs:** Mastodon, Bluesky, Dev.to
2. **Use Twitter free tier** (500 writes/month is enough); upgrade to Basic only if you need analytics
3. **Consider Publer ($12/mo)** if you want visual scheduling for Twitter/LinkedIn/Facebook/Mastodon/Bluesky in one dashboard
4. **Build custom adapters** for Dev.to, Medium, and Bluesky (simple REST, ~30 min each)
5. **Never fully automate Reddit** -- prepare content, submit manually
6. **Manual for HN, Lobsters, Tildes, LtU, Discord** -- no shortcuts here
7. **LinkedIn requires the most OAuth work** -- consider Publer to avoid building the token refresh flow
8. **Facebook is the most friction for least reward** -- deprioritize unless you have an active Page audience

Total estimated implementation time: **8-12 hours** for the full custom script with all adapters, or **2-3 hours** if using Publer for the major social platforms and only building Dev.to + Medium adapters.
