# مرجع الواجهات والتخزين

هذا مرجع تقني للتكامل بين الواجهة وخدمة Rust وقاعدة PostgreSQL ولوحة
FastAPI. لا توجد واجهة Cloud HTTP عامة في التصميم الحالي.

## خدمة Rust المحلية

العنوان الافتراضي:

```text
http://127.0.0.1:45632
```

تستعملها واجهة `ActivateurRMS.exe` فقط على جهاز المولد.

### `GET /health`

الاستجابة:

```json
{
  "status": "ok",
  "backend_mode": "local-queue-v1",
  "authorized": true,
  "maintenance": false,
  "pending_uploads": 0
}
```

| الحقل | المعنى |
| --- | --- |
| `backend_mode` | إصدار عقد backend الذي تتطلبه الواجهة |
| `authorized` | وجود علامة تفويض التشغيل الأول `AUTHORIZED.txt` |
| `maintenance` | هل التوليد ممنوع محليا بسبب الحالة `0` |
| `pending_uploads` | عدد السجلات التي لم تنظف من طابور الرفع بعد |

تتحقق الواجهة أيضا من أن خدمة Windows `KeyGenService` مسجلة، فلا يكفي تشغيل
خادم يدوي على المنفذ نفسه.

### `POST /generate_key`

الطلب:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "app_type": "Restaurant"
}
```

القيم المقبولة لـ `app_type`:

```text
Restaurant
Lab
Jewelry
```

الاستجابة الناجحة:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "activation_key": "EE8C-551F-0A90-73F5",
  "app_type": "Restaurant",
  "status": "generated_and_queued"
}
```

معنى `generated_and_queued`: المفتاح أنشئ وحفظ محليا، وأضيف سجل إلى طابور
المزامنة؛ قد يكون الرفع إلى PostgreSQL تم لاحقا.

الأخطاء المهمة:

| HTTP | السبب |
| --- | --- |
| `400` | JSON غير صالح، كود طلب غير صحيح، أو نوع منتج غير معروف |
| `503` | وضع الصيانة فعال أو فشل حفظ العملية محليا |

## أوامر تنفيذية داخلية/إدارية

### أوامر الواجهة

```powershell
ActivateurRMS.exe --install
ActivateurRMS.exe --uninstall
ActivateurRMS.exe --unstall
ActivateurRMS.exe --help
```

### أمر backend المستخدم أثناء التثبيت

```powershell
KeyGenService.exe --authorize-install
```

يتصل هذا الأمر بقاعدة البيانات ويخرج بنجاح فقط إذا كان
`server_control.status='1'`. تستخدمه الواجهة قبل تثبيت أو تحديث الخدمة.

## إعداد خدمة Rust

| المتغير | مطلوب | القيمة الافتراضية | الوظيفة |
| --- | --- | --- | --- |
| `PGHOST` | نعم | لا يوجد | مضيف PostgreSQL |
| `PGPORT` | نعم | لا يوجد | منفذ PostgreSQL |
| `PGDATABASE` | نعم | لا يوجد | اسم قاعدة البيانات |
| `PGUSER` | نعم | لا يوجد | حساب المولد المحدود |
| `PGPASSWORD` | نعم | لا يوجد | كلمة المرور |
| `PGSSLMODE` | لا | `require` | `disable` أو `require` أو `verify-full` |
| `PGCONNECT_TIMEOUT` | لا | `10` | مهلة الاتصال بالثواني |
| `KEYGEN_LISTEN_ADDRESS` | لا | `127.0.0.1:45632` | عنوان HTTP المحلي |
| `KEYGEN_STATUS_INTERVAL_SECONDS` | لا | `15` | دورية قراءة الحالة |
| `KEYGEN_UPLOAD_INTERVAL_SECONDS` | لا | `5` | دورية محاولة الرفع |
| `KEYGEN_DATA_DIR` | لا | `%ProgramData%\KeyGenRMS` | مجلد البيانات |
| `KEYGEN_PRIMARY_LOG` | لا | `generated_keys.txt` داخل مجلد البيانات | سجل المفاتيح الأساسي |

يتضمن البناء القيم غير السرية الممكنة فقط. تأتي `PGUSER` و`PGPASSWORD`
من `.env` المحلي أثناء التشغيل/التثبيت.

## التخزين المحلي

| الملف | يكتبه | الاستخدام |
| --- | --- | --- |
| `generated_keys.txt` | خدمة Rust | سجل نصي لكل مفتاح مولد |
| `netcache.dat` | خدمة Rust | سجل إضافي محلي |
| `pending_uploads.json` | خدمة Rust | طابور السجلات قيد الرفع |
| `uploaded.log` | خدمة Rust | أثر نجاح الرفع |
| `AUTHORIZED.txt` | خدمة Rust | التفويض الأولي |
| `MAINTENANCE.txt` | خدمة Rust | حالة الإيقاف المستلمة |

كل عنصر في الطابور يحتوي `sync_id` لمنع إدخال العملية نفسها مرتين عند إعادة
محاولة المزامنة.

## عقد PostgreSQL

### جدول الحالة

```sql
CREATE TABLE IF NOT EXISTS server_control (
    id INTEGER PRIMARY KEY,
    status TEXT NOT NULL CHECK (status IN ('0', '1'))
);

INSERT INTO server_control (id, status)
VALUES (1, '1')
ON CONFLICT (id) DO NOTHING;
```

تقرأ خدمة Rust:

```sql
SELECT status FROM server_control WHERE id = 1;
```

### جدول السجلات

```sql
CREATE TABLE IF NOT EXISTS activation_logs (
    id BIGSERIAL PRIMARY KEY,
    sync_id TEXT UNIQUE,
    request_code TEXT,
    activation_key TEXT,
    generated_at TEXT,
    device_ip TEXT
);
```

تزامن خدمة Rust:

```sql
INSERT INTO activation_logs
    (sync_id, request_code, activation_key, generated_at, device_ip)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT DO NOTHING;
```

`ON CONFLICT DO NOTHING` يدعم إعادة المحاولة سواء كان القيد الفريد الأصلي
أو الفهرس الجزئي على `sync_id` موجودا في قاعدة تمت ترقيتها.

## واجهة FastAPI المحلية

العنوان الافتراضي:

```text
http://127.0.0.1:8080
```

| المسار | الطلب | النتيجة |
| --- | --- | --- |
| `/` | `GET` | صفحة المتصفح |
| `/health` | `GET` | حالة تشغيل الموقع |
| `/api/status` | `GET` | `{"status":"1"}` أو `{"status":"0"}` |
| `/api/status` | `PUT {"status":"0"}` | تغيير حالة التحكم |
| `/api/activation-logs?limit=100` | `GET` | أحدث سجلات PostgreSQL |

قيمة `limit` من `1` إلى `500`. ترفض اللوحة الاتصالات غير المحلية افتراضيا
ما لم يضبط `ADMIN_WEB_ALLOW_REMOTE=1`، وهو إعداد لا ينصح به دون حماية إضافية.
