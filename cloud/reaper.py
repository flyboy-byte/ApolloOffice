#!/usr/bin/env python3
"""L3 dead-man switch for rented ApolloOffice boxes (fork; docs/cloud-hosts.md). Runs on the VPS from cron, so boxes die
even when the laptop is off, asleep or crashed. Deletes every Linode tagged apollo-office whose ao-exp-<unix> tag is more
than GRACE seconds in the past. Dry run unless --apply.

  */5 * * * * LINODE_TOKEN_FILE=/path/to/token /usr/bin/python3 reaper.py --apply >> reaper.log 2>&1

The token needs read/write on Linodes only.
"""
import json, os, sys, time, urllib.request

API = os.environ.get("LINODE_API", "https://api.linode.com/v4")
GRACE = int(os.environ.get("GRACE", "1800"))


def token():
    f = os.environ.get("LINODE_TOKEN_FILE")
    t = open(f).read().strip() if f else os.environ.get("LINODE_TOKEN", "").strip()
    if not t:
        sys.exit("set LINODE_TOKEN_FILE or LINODE_TOKEN")
    return t


def call(method, path, tok, flt=None):
    h = {"authorization": "Bearer " + tok}
    if flt:
        h["X-Filter"] = json.dumps(flt)
    with urllib.request.urlopen(urllib.request.Request(API + path, method=method, headers=h), timeout=30) as r:
        b = r.read()
        return json.loads(b) if b else {}


def expired(tags, now):
    for t in tags:
        if t.startswith("ao-exp-") and t[7:].isdigit():
            return now > int(t[7:]) + GRACE
    return False


def main():
    apply = "--apply" in sys.argv
    tok, now = token(), int(time.time())
    data = call("GET", "/linode/instances?page_size=500", tok, {"tags": "apollo-office"}).get("data", [])
    for i in data:
        if expired(i.get("tags", []), now):
            print(time.strftime("%F %T"), ("DELETE" if apply else "would delete"), i["id"], i.get("label"), i.get("tags"), flush=True)
            if apply:
                call("DELETE", f"/linode/instances/{i['id']}", tok)


if __name__ == "__main__":
    main()
