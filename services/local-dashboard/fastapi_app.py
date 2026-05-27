import argparse
import os

import uvicorn

from dashboard_app.admin_web import create_admin_app
from dashboard_app.config import load_environment


app = create_admin_app()


def parse_args():
    parser = argparse.ArgumentParser(description="Local KeyGen PostgreSQL dashboard")
    parser.add_argument(
        "--host",
        default=os.environ.get("ADMIN_WEB_HOST", "127.0.0.1"),
        help="Listen address. Defaults to the local computer only.",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=int(os.environ.get("ADMIN_WEB_PORT", "8080")),
        help="Listen port. Defaults to 8080.",
    )
    parser.add_argument(
        "--reload",
        action="store_true",
        help="Restart automatically when source files change during development.",
    )
    return parser.parse_args()


def main():
    load_environment()
    args = parse_args()
    uvicorn.run(
        "fastapi_app:app",
        host=args.host,
        port=args.port,
        reload=args.reload,
    )


if __name__ == "__main__":
    main()
