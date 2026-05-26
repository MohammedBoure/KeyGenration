# لوحة المراقبة المحلية بـ FastAPI

توفر هذه الخدمة صفحة ويب محلية لعرض سجلات التفعيل من قاعدة PostgreSQL
الفعلية وتغيير حالة توليد المفاتيح بين:

- `1`: يسمح Cloud API بإصدار المفاتيح لبرنامج العميل.
- `0`: يرفض Cloud API إصدار أي مفتاح وتمنع واجهة العميل الجديدة التشغيل.

تعمل اللوحة بشكل مستقل عن واجهة Windows، ولكنها تستخدم نفس جدولي
`server_control` و`activation_logs` اللذين يستخدمهما Cloud API.

## التشغيل

من جذر المشروع:

```powershell
cd .\services\cloud-api
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
```

حرر الملف `.env` وضع بيانات PostgreSQL الفعلية. لا تحفظ كلمة المرور في Git:

```dotenv
PGHOST=sw4.duckdns.org
PGPORT=9005
PGDATABASE=keygen_restaurant
PGUSER=keygen_app
PGPASSWORD=replace-with-database-password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10
ADMIN_WEB_HOST=127.0.0.1
ADMIN_WEB_PORT=8080
```

شغل الخدمة:

```powershell
python .\fastapi_app.py
```

ثم افتح في المتصفح المحلي:

```text
http://127.0.0.1:8080/
```

عند بدء التشغيل تهيئ الخدمة الجداول المطلوبة إن لم تكن موجودة، ثم تقرأ
السجلات من قاعدة البيانات المعرفة في `.env`. تتحدث الصفحة تلقائيا كل
15 ثانية، ويمكن تغيير حالة البرنامج من زري `تشغيل (1)` و`إيقاف (0)`.

## مسارات الخدمة

| المسار | الوظيفة |
| --- | --- |
| `GET /` | صفحة المراقبة والتحكم |
| `GET /health` | التحقق من عمل خدمة الويب |
| `GET /api/status` | قراءة القيمة الحالية من `server_control` |
| `PUT /api/status` | كتابة `{"status":"0"}` أو `{"status":"1"}` |
| `GET /api/activation-logs?limit=100` | قراءة أحدث السجلات من `activation_logs` |
| `GET /docs` | واجهة توثيق FastAPI التفاعلية |

## الأمان

تتصل عملية Python بقاعدة البيانات مباشرة، ولا ترسل كلمة مرور PostgreSQL
إلى المتصفح. تستمع الخدمة إلى `127.0.0.1` افتراضيا، وترفض الطلبات القادمة
من جهاز آخر ما لم يضبط المشغل عمدا `ADMIN_WEB_ALLOW_REMOTE=1`.

لا تعرض هذه اللوحة على الإنترنت مباشرة؛ استخدم Cloud API المنشور خلف HTTPS
للوصول البعيد، أو أضف مصادقة وHTTPS قبل السماح بالوصول الشبكي للوحة.
