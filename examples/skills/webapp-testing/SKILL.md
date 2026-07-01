---
name: {{ skill_name }}
description: {{ skill_description }}
license: {{ license }}
---

# {{ skill_name }}

{{ skill_description }}

Ported from Anthropic's public `webapp-testing` Agent Skill — write native Python
Playwright scripts rather than reaching for a heavier test framework.

## When to use

{{ when_to_use }}

**Helper script available**: `scripts/with_server.py` manages server lifecycle
(supports starting multiple servers before running an automation script).

**Always run `python scripts/with_server.py --help` first.** Don't read its source
until a customized solution turns out to be necessary — it exists to be called as a
black-box script, not ingested into context.

## Decision tree

```
Is it static HTML?
  Yes -> read the HTML file directly to find selectors, then write a Playwright
         script against them
  No  -> is the dev server already running?
           No  -> python scripts/with_server.py --server "..." --port 5173 -- python automation.py
           Yes -> reconnaissance-then-action:
                  1. navigate, wait_for_load_state('networkidle')
                  2. screenshot or inspect the DOM
                  3. identify selectors from the rendered state
                  4. execute actions with those selectors
```

## Reconnaissance-then-action pattern

```python
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()
    page.goto('http://localhost:5173')
    page.wait_for_load_state('networkidle')  # CRITICAL: wait before inspecting
    page.screenshot(path='/tmp/inspect.png', full_page=True)
    browser.close()
```

❌ Don't inspect the DOM before waiting for `networkidle` on dynamic apps.
✅ Do wait for `page.wait_for_load_state('networkidle')` before inspection.

## Reference examples

`examples/` holds three worked Playwright patterns copied verbatim from the upstream
skill:
- `element_discovery.py` — enumerating buttons, links, and inputs on a page.
- `static_html_automation.py` — driving local HTML via `file://` URLs.
- `console_logging.py` — capturing browser console output during a run.

{{ harness.subagent_guide }}
