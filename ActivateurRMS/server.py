import sys
from contextlib import contextmanager

import psycopg2
from psycopg2.extras import RealDictCursor
from flask import Flask, request, jsonify, render_template_string
from functools import wraps
import os

app = Flask(__name__)

DEFAULT_DB_HOST = "sw4.duckdns.org"
DEFAULT_DB_PORT = "9005"
DEFAULT_DB_NAME = "keygen_restaurant"
DEFAULT_DB_USER = "keygen_app"


def get_db_connection():
    options = {
        "sslmode": os.environ.get("PGSSLMODE", "require"),
        "connect_timeout": int(os.environ.get("PGCONNECT_TIMEOUT", "10")),
    }
    database_url = os.environ.get("DATABASE_URL")
    if database_url:
        return psycopg2.connect(database_url, **options)

    password = os.environ.get("PGPASSWORD")
    if not password:
        raise RuntimeError("PGPASSWORD is required to connect to PostgreSQL.")

    return psycopg2.connect(
        host=os.environ.get("PGHOST", DEFAULT_DB_HOST),
        port=os.environ.get("PGPORT", DEFAULT_DB_PORT),
        dbname=os.environ.get("PGDATABASE", DEFAULT_DB_NAME),
        user=os.environ.get("PGUSER", DEFAULT_DB_USER),
        password=password,
        **options,
    )


@contextmanager
def db_cursor(dict_rows=False):
    conn = get_db_connection()
    cursor = conn.cursor(cursor_factory=RealDictCursor) if dict_rows else conn.cursor()
    try:
        yield cursor
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        cursor.close()
        conn.close()

def init_db():
    with db_cursor() as cursor:
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS server_control (
                id INTEGER PRIMARY KEY,
                status TEXT NOT NULL CHECK (status IN ('0', '1'))
            )
        ''')

        cursor.execute('''
            CREATE TABLE IF NOT EXISTS activation_logs (
                id BIGSERIAL PRIMARY KEY,
                request_code TEXT,
                activation_key TEXT,
                generated_at TEXT,
                device_ip TEXT
            )
        ''')

        cursor.execute('''
            INSERT INTO server_control (id, status)
            VALUES (1, '1')
            ON CONFLICT (id) DO NOTHING
        ''')

init_db()


def migrate_from_sqlite(sqlite_path):
    import sqlite3

    source = sqlite3.connect(sqlite_path)
    source.row_factory = sqlite3.Row
    try:
        status_rows = source.execute(
            'SELECT id, status FROM server_control'
        ).fetchall()
        log_rows = source.execute('''
            SELECT id, request_code, activation_key, generated_at, device_ip
            FROM activation_logs
            ORDER BY id
        ''').fetchall()
    finally:
        source.close()

    with db_cursor() as cursor:
        for row in status_rows:
            cursor.execute('''
                INSERT INTO server_control (id, status)
                VALUES (%s, %s)
                ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status
            ''', (row['id'], row['status']))

        for row in log_rows:
            cursor.execute('''
                INSERT INTO activation_logs
                    (id, request_code, activation_key, generated_at, device_ip)
                VALUES (%s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
            ''', (
                row['id'],
                row['request_code'],
                row['activation_key'],
                row['generated_at'],
                row['device_ip'],
            ))

        cursor.execute('''
            SELECT setval(
                pg_get_serial_sequence('activation_logs', 'id'),
                COALESCE((SELECT MAX(id) FROM activation_logs), 1),
                EXISTS (SELECT 1 FROM activation_logs)
            )
        ''')

    return len(log_rows)

def require_auth(f):
    @wraps(f)
    def decorated(*args, **kwargs):
        api_secret_token = os.environ.get("KEYGEN_API_SECRET_TOKEN")
        if not api_secret_token:
            return jsonify({"error": "Server authentication is not configured."}), 500

        auth_header = request.headers.get('Authorization')
        if not auth_header or not auth_header.startswith("Bearer "):
            return jsonify({"error": "Unauthorized. Missing or invalid token."}), 401

        token = auth_header.split(" ")[1]
        if token != api_secret_token:
            return jsonify({"error": "Forbidden. Invalid token."}), 403

        return f(*args, **kwargs)
    return decorated

@app.route('/api/v1/server-status', methods=['GET'])
@require_auth
def get_server_status():
    try:
        with db_cursor() as cursor:
            cursor.execute('SELECT status FROM server_control WHERE id = 1')
            result = cursor.fetchone()

        if result:
            return jsonify({"status": result[0]}), 200
        else:
            return jsonify({"status": "0", "error": "Status record not found"}), 500
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/v1/set-status', methods=['POST'])
@require_auth
def set_server_status():
    data = request.json
    new_status = str(data.get("status", "")).strip()

    if new_status not in ["0", "1"]:
        return jsonify({"error": "Invalid status. Use '1' (Active) or '0' (Maintenance)."}), 400

    try:
        with db_cursor() as cursor:
            cursor.execute(
                'UPDATE server_control SET status = %s WHERE id = 1',
                (new_status,)
            )
        return jsonify({"message": f"Server status updated to {new_status}"}), 200
    except Exception as e:
        return jsonify({"error": str(e)}), 500

# تم التعديل هنا: إضافة GET لجلب البيانات للوحة التحكم
@app.route('/api/v1/activation-logs', methods=['POST', 'GET'])
@require_auth
def handle_logs():
    if request.method == 'POST':
        records = request.json
        if not isinstance(records, list):
            return jsonify({"error": "Expected a list of JSON objects"}), 400

        try:
            with db_cursor() as cursor:
                for item in records:
                    cursor.execute('''
                        INSERT INTO activation_logs
                            (request_code, activation_key, generated_at, device_ip)
                        VALUES (%s, %s, %s, %s)
                    ''', (
                        item.get("request_code"),
                        item.get("activation_key"),
                        item.get("generated_at"),
                        item.get("device_ip")
                    ))

            return jsonify({"message": f"Successfully inserted {len(records)} records."}), 201
        except Exception as e:
            return jsonify({"error": str(e)}), 500

    elif request.method == 'GET':
        try:
            with db_cursor(dict_rows=True) as cursor:
                cursor.execute('SELECT * FROM activation_logs ORDER BY id DESC')
                rows = cursor.fetchall()

            logs = [dict(row) for row in rows]
            return jsonify(logs), 200
        except Exception as e:
            return jsonify({"error": str(e)}), 500

# مسار جديد: واجهة الويب (لوحة التحكم)
@app.route('/dashboard')
def admin_dashboard():
    html_content = """
    <!DOCTYPE html>
    <html lang="ar" dir="rtl">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>لوحة تحكم التفعيلات</title>
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.rtl.min.css" rel="stylesheet">
        <style>
            body { background-color: #f8f9fa; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; }
            .card { border-radius: 15px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); margin-bottom: 20px; }
            .status-badge { font-size: 1.2em; padding: 10px 20px; border-radius: 10px; }
        </style>
    </head>
    <body>
        <div class="container mt-5">
            <h1 class="text-center mb-4 text-primary">لوحة تحكم السيرفر (KeyGen)</h1>

            <div class="card p-4">
                <div class="row align-items-end">
                    <div class="col-md-8">
                        <label class="form-label fw-bold">كلمة المرور (API Token):</label>
                        <input type="password" id="apiToken" class="form-control" placeholder="أدخل التوكن السري للاتصال...">
                    </div>
                    <div class="col-md-4 mt-3 mt-md-0">
                        <button class="btn btn-primary w-100" onclick="loadData()">دخول وتحديث البيانات</button>
                    </div>
                </div>
                <div id="authAlert" class="alert alert-danger mt-3 d-none">التوكن غير صحيح!</div>
            </div>

            <div id="mainContent" class="d-none">
                <div class="card p-4 text-center">
                    <h3>حالة التثبيت التلقائي</h3>
                    <div class="my-3">
                        <span id="statusIndicator" class="badge bg-secondary status-badge">جاري التحميل...</span>
                    </div>
                    <div>
                        <button class="btn btn-success me-2" onclick="changeStatus('1')">تشغيل السيرفر (1)</button>
                        <button class="btn btn-danger" onclick="changeStatus('0')">إيقاف / صيانة (0)</button>
                    </div>
                </div>

                <div class="card p-4">
                    <h3>سجلات التفعيل (Logs)</h3>
                    <div class="table-responsive mt-3">
                        <table class="table table-hover table-bordered">
                            <thead class="table-dark">
                                <tr>
                                    <th>#</th>
                                    <th>كود الطلب</th>
                                    <th>مفتاح التفعيل</th>
                                    <th>عنوان IP</th>
                                    <th>تاريخ التوليد</th>
                                </tr>
                            </thead>
                            <tbody id="logsTableBody">
                                </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </div>

        <script>
            let currentStatus = "0";

            function getHeaders() {
                const token = document.getElementById('apiToken').value;
                return {
                    "Authorization": "Bearer " + token,
                    "Content-Type": "application/json"
                };
            }

            async function loadData() {
                const token = document.getElementById('apiToken').value;
                if (!token) { alert("الرجاء إدخال التوكن!"); return; }

                document.getElementById('authAlert').classList.add('d-none');

                try {
                    // جلب حالة السيرفر
                    let statusRes = await fetch('/api/v1/server-status', { headers: getHeaders() });
                    if (statusRes.status === 401 || statusRes.status === 403) {
                        document.getElementById('authAlert').classList.remove('d-none');
                        document.getElementById('mainContent').classList.add('d-none');
                        return;
                    }

                    let statusData = await statusRes.json();
                    updateStatusUI(statusData.status);

                    // جلب السجلات
                    let logsRes = await fetch('/api/v1/activation-logs', { headers: getHeaders() });
                    let logsData = await logsRes.json();

                    let tbody = document.getElementById('logsTableBody');
                    tbody.innerHTML = "";

                    logsData.forEach(log => {
                        tbody.innerHTML += `
                            <tr>
                                <td>${log.id}</td>
                                <td><span class="badge bg-info text-dark">${log.request_code}</span></td>
                                <td><code>${log.activation_key}</code></td>
                                <td>${log.device_ip}</td>
                                <td><small class="text-muted">${log.generated_at}</small></td>
                            </tr>
                        `;
                    });

                    document.getElementById('mainContent').classList.remove('d-none');

                } catch (error) {
                    console.error("Error:", error);
                    alert("حدث خطأ في الاتصال بالسيرفر.");
                }
            }

            async function changeStatus(newStatus) {
                if(!confirm(`هل أنت متأكد من تغيير حالة السيرفر إلى ${newStatus == '1' ? 'تشغيل' : 'إيقاف'}؟`)) return;

                try {
                    let res = await fetch('/api/v1/set-status', {
                        method: 'POST',
                        headers: getHeaders(),
                        body: JSON.stringify({ status: newStatus })
                    });

                    if(res.ok) {
                        updateStatusUI(newStatus);
                    } else {
                        alert("فشل في تحديث الحالة.");
                    }
                } catch (error) {
                    console.error("Error:", error);
                }
            }

            function updateStatusUI(status) {
                let indicator = document.getElementById('statusIndicator');
                if (status === "1") {
                    indicator.textContent = "يعمل (Active)";
                    indicator.className = "badge bg-success status-badge";
                } else {
                    indicator.textContent = "متوقف / صيانة (Maintenance)";
                    indicator.className = "badge bg-danger status-badge";
                }
            }
        </script>
    </body>
    </html>
    """
    return render_template_string(html_content)

if __name__ == '__main__' and len(sys.argv) == 3 and sys.argv[1] == '--migrate-sqlite':
    migrated_count = migrate_from_sqlite(sys.argv[2])
    print(f"Imported or retained {migrated_count} SQLite activation log records.")
elif __name__ == '__main__':
    app.run(
        host='0.0.0.0',
        port=7002,
        debug=os.environ.get("FLASK_DEBUG", "0") == "1"
    )
