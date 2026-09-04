#!/usr/bin/env python3
"""
GitHub activity fetcher for the lookingback skill.

Fetches PRs, Issues, and comments for a given user in a GitHub org
within a specified date range using the GitHub REST API.

Usage:
    python3 fetch_github.py --username kwmt --org appbrew \
        --start-date 2026-03-13 --end-date 2026-03-27

Requires GITHUB_TOKEN environment variable to be set.
"""

import argparse
import json
import os
import sys
import urllib.request
import urllib.error
import urllib.parse
from datetime import datetime


def github_api_request(url, token):
    """Make a GitHub API request and return parsed JSON."""
    headers = {
        "Accept": "application/vnd.github.v3+json",
        "User-Agent": "lookingback-skill",
    }
    if token:
        headers["Authorization"] = f"token {token}"

    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8") if e.fp else ""
        return {"error": f"HTTP {e.code}: {e.reason}", "details": error_body}
    except urllib.error.URLError as e:
        return {"error": f"URL Error: {str(e)}"}


def search_github(query, token, search_type="issues"):
    """Search GitHub using the search API. Returns all results with pagination."""
    all_items = []
    page = 1
    per_page = 100

    while True:
        encoded_query = urllib.parse.quote(query)
        url = f"https://api.github.com/search/{search_type}?q={encoded_query}&per_page={per_page}&page={page}&sort=created&order=desc"
        result = github_api_request(url, token)

        if "error" in result:
            return result

        items = result.get("items", [])
        all_items.extend(items)

        if len(items) < per_page:
            break
        page += 1
        if page > 10:  # Safety limit
            break

    return {"total_count": len(all_items), "items": all_items}


def fetch_comments(url, token, start_date, end_date):
    """Fetch comments from a given GitHub API comments endpoint."""
    all_comments = []
    page = 1
    per_page = 100

    while True:
        paginated_url = f"{url}?since={start_date}T00:00:00Z&per_page={per_page}&page={page}&sort=created&direction=desc"
        result = github_api_request(paginated_url, token)

        if isinstance(result, dict) and "error" in result:
            return result

        if not isinstance(result, list):
            break

        # Filter by end date
        filtered = []
        for c in result:
            created = c.get("created_at", "")[:10]
            if created <= end_date:
                filtered.append(c)

        all_comments.extend(filtered)

        if len(result) < per_page:
            break
        page += 1
        if page > 10:
            break

    return all_comments


def extract_repo_name(item):
    """Extract repository name from a search result item."""
    repo_url = item.get("repository_url", "")
    if repo_url:
        return repo_url.split("/")[-1]
    html_url = item.get("html_url", "")
    parts = html_url.split("/")
    for i, p in enumerate(parts):
        if p == "github.com" and i + 2 < len(parts):
            return parts[i + 2]
    return "unknown"


def main():
    parser = argparse.ArgumentParser(description="Fetch GitHub activity for lookingback skill")
    parser.add_argument("--username", required=True, help="GitHub username")
    parser.add_argument("--org", required=True, help="GitHub organization")
    parser.add_argument("--start-date", required=True, help="Start date (YYYY-MM-DD)")
    parser.add_argument("--end-date", required=True, help="End date (YYYY-MM-DD)")
    args = parser.parse_args()

    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if not token:
        print(json.dumps({
            "error": "GITHUB_TOKEN or GH_TOKEN environment variable is not set.",
            "help": "Please create a GitHub Personal Access Token at https://github.com/settings/tokens and set it as GITHUB_TOKEN environment variable."
        }))
        sys.exit(1)

    date_range = f"{args.start_date}..{args.end_date}"
    output = {
        "username": args.username,
        "org": args.org,
        "period": {"start": args.start_date, "end": args.end_date},
        "pull_requests": [],
        "issues": [],
        "pr_review_comments": [],
        "issue_comments": [],
    }

    # 1. Fetch PRs authored by user in org
    pr_query = f"org:{args.org} author:{args.username} created:{date_range} is:pr"
    pr_results = search_github(pr_query, token)
    if "error" not in pr_results:
        for item in pr_results.get("items", []):
            output["pull_requests"].append({
                "title": item.get("title"),
                "url": item.get("html_url"),
                "repository": extract_repo_name(item),
                "state": item.get("state"),
                "created_at": item.get("created_at", "")[:10],
                "merged": item.get("pull_request", {}).get("merged_at") is not None if item.get("pull_request") else False,
                "number": item.get("number"),
            })
    else:
        output["pull_requests_error"] = pr_results["error"]

    # 2. Fetch Issues authored by user in org
    issue_query = f"org:{args.org} author:{args.username} created:{date_range} is:issue"
    issue_results = search_github(issue_query, token)
    if "error" not in issue_results:
        for item in issue_results.get("items", []):
            output["issues"].append({
                "title": item.get("title"),
                "url": item.get("html_url"),
                "repository": extract_repo_name(item),
                "state": item.get("state"),
                "created_at": item.get("created_at", "")[:10],
                "number": item.get("number"),
            })
    else:
        output["issues_error"] = issue_results["error"]

    # 3. Fetch PR review comments by user in org
    pr_comment_query = f"org:{args.org} commenter:{args.username} created:{date_range} is:pr"
    pr_comment_results = search_github(pr_comment_query, token)
    if "error" not in pr_comment_results:
        for item in pr_comment_results.get("items", []):
            # Skip if this is the user's own PR (already captured above)
            if item.get("user", {}).get("login") == args.username:
                continue
            output["pr_review_comments"].append({
                "pr_title": item.get("title"),
                "pr_url": item.get("html_url"),
                "repository": extract_repo_name(item),
                "number": item.get("number"),
            })
    else:
        output["pr_review_comments_error"] = pr_comment_results["error"]

    # 4. Fetch Issue comments by user in org
    issue_comment_query = f"org:{args.org} commenter:{args.username} created:{date_range} is:issue"
    issue_comment_results = search_github(issue_comment_query, token)
    if "error" not in issue_comment_results:
        for item in issue_comment_results.get("items", []):
            if item.get("user", {}).get("login") == args.username:
                continue
            output["issue_comments"].append({
                "issue_title": item.get("title"),
                "issue_url": item.get("html_url"),
                "repository": extract_repo_name(item),
                "number": item.get("number"),
            })
    else:
        output["issue_comments_error"] = issue_comment_results["error"]

    # Summary counts
    output["summary"] = {
        "total_prs": len(output["pull_requests"]),
        "total_issues": len(output["issues"]),
        "total_pr_reviews": len(output["pr_review_comments"]),
        "total_issue_comments": len(output["issue_comments"]),
    }

    print(json.dumps(output, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
