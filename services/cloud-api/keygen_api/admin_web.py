import logging
import os
from contextlib import asynccontextmanager
from ipaddress import ip_address
from pathlib import Path
from typing import Literal

from fastapi import FastAPI, HTTPException, Path as PathParameter, Query, Request
from fastapi.responses import FileResponse, JSONResponse
from pydantic import BaseModel

from . import config, db


LOGGER = logging.getLogger(__name__)
DASHBOARD_PAGE = Path(__file__).resolve().parent / "templates" / "local_dashboard.html"


class StatusUpdate(BaseModel):
    status: Literal["0", "1"]


def _is_local_client(request: Request):
    client_host = request.client.host if request.client else ""
    if client_host == "testclient":
        return True
    try:
        return ip_address(client_host).is_loopback
    except ValueError:
        return False


def _database_error(operation, error):
    LOGGER.exception("PostgreSQL %s failed.", operation)
    raise HTTPException(
        status_code=503,
        detail="تعذر الاتصال بقاعدة بيانات PostgreSQL. تحقق من إعدادات .env.",
    ) from error


def create_admin_app(initialize_database=True):
    @asynccontextmanager
    async def lifespan(_application):
        config.load_environment()
        if initialize_database:
            db.init_db()
        yield

    application = FastAPI(
        title="KeyGen Local Administration Dashboard",
        description="Local PostgreSQL monitor and generation status controller.",
        lifespan=lifespan,
    )

    @application.middleware("http")
    async def restrict_to_local_computer(request, call_next):
        if os.environ.get("ADMIN_WEB_ALLOW_REMOTE") != "1" and not _is_local_client(
            request
        ):
            return JSONResponse(
                status_code=403,
                content={
                    "detail": "This dashboard only accepts local requests by default."
                },
            )
        return await call_next(request)

    @application.get("/", include_in_schema=False)
    def dashboard_page():
        return FileResponse(DASHBOARD_PAGE)

    @application.get("/health")
    def health():
        return {"status": "ok", "service": "keygen-admin-web"}

    @application.get("/api/status")
    def read_status():
        try:
            with db.db_cursor() as cursor:
                cursor.execute("SELECT status FROM server_control WHERE id = 1")
                row = cursor.fetchone()
        except Exception as error:
            _database_error("status read", error)

        if not row:
            raise HTTPException(
                status_code=500,
                detail="لم يتم العثور على سجل حالة التشغيل.",
            )
        return {"status": row[0]}

    @application.put("/api/status")
    def write_status(payload: StatusUpdate):
        try:
            with db.db_cursor() as cursor:
                cursor.execute(
                    """
                    INSERT INTO server_control (id, status)
                    VALUES (1, %s)
                    ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status
                    """,
                    (payload.status,),
                )
        except Exception as error:
            _database_error("status update", error)

        return {"status": payload.status}

    @application.get("/api/activation-logs")
    def read_activation_logs(limit: int = Query(default=100, ge=1, le=500)):
        try:
            with db.db_cursor(dict_rows=True) as cursor:
                cursor.execute(
                    """
                    SELECT id, request_code, activation_key, generated_at, device_ip
                    FROM activation_logs
                    ORDER BY id DESC
                    LIMIT %s
                    """,
                    (limit,),
                )
                rows = cursor.fetchall()
        except Exception as error:
            _database_error("activation log read", error)

        records = [dict(row) for row in rows]
        return {"records": records, "count": len(records)}

    @application.delete("/api/activation-logs/{log_id}")
    def delete_activation_log(log_id: int = PathParameter(..., ge=1)):
        try:
            with db.db_cursor() as cursor:
                cursor.execute(
                    "DELETE FROM activation_logs WHERE id = %s RETURNING id",
                    (log_id,),
                )
                deleted = cursor.fetchone()
        except Exception as error:
            _database_error("activation log delete", error)

        if not deleted:
            raise HTTPException(
                status_code=404,
                detail="لم يتم العثور على سجل المفتاح المطلوب حذفه.",
            )

        return {"deleted": True, "id": log_id}

    return application
