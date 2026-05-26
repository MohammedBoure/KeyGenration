# البناء والنشر

## البنية

```text
apps/desktop/            مصدر واجهة ActivateurRMS.exe
apps/keygen-service/     مصدر خدمة KeyGenService.exe
crates/config/           قارئ .env والأنواع المشتركة
services/cloud-api/      API السحابي ولوحة التحكم
packaging/windows/       تغليف عميل Windows وNSSM
dist/                    ناتج البناء المحلي غير المتتبع
```

## بناء عميل Rust

من جذر المستودع:

```powershell
cargo fmt --all
cargo test
cargo clippy --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

يشغل ملف `package.cmd` سكربت PowerShell بسياسة تنفيذ مناسبة للتغليف المحلي،
ثم ينفذ `cargo build --release --workspace` وينسخ ملفات التشغيل إلى
`dist\windows\ActivateurRMS\`، ثم ينشئ `SHA256SUMS.txt`.

## تشغيل Cloud API

```powershell
cd .\services\cloud-api
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# حرر .env وضع كلمة مرور PostgreSQL وتوكن API.
python .\app.py
```

ملف `.env` الخاص بالخادم:

```dotenv
PGHOST=sw4.duckdns.org
PGPORT=9005
PGDATABASE=keygen_restaurant
PGUSER=keygen_app
PGPASSWORD=replace-with-database-password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10
KEYGEN_API_SECRET_TOKEN=replace-with-api-token
FLASK_DEBUG=0
```

يستمع التشغيل المباشر على `http://0.0.0.0:7002`. في الإنتاج ضع الخدمة خلف
HTTPS واستخدم عنوان HTTPS في حزمة العميل.

بعد تشغيل تهيئة الجداول، يمكن لخادم WSGI استيراد التطبيق من `wsgi.py`
باسم `application`.

## تهيئة قاعدة البيانات

ينشئ `python .\app.py` تلقائيا جدول التحكم وجدول سجلات التفعيل. لتهيئة
الجداول دون تشغيل HTTP:

```powershell
python .\app.py --init-db-only
```

## استيراد SQLite

```powershell
python .\app.py --migrate-sqlite .\cloud_database.db
```

الاستيراد لا يكرر سجلات التفعيل التي تحمل معرفا موجودا.

## لوحة التحكم

بعد نشر API افتح:

```text
https://your-api-host/dashboard
```

أدخل `KEYGEN_API_SECRET_TOKEN` لقراءة السجلات أو تغيير وضع التوليد.

للمراقبة المباشرة من جهاز الإدارة دون نشر Cloud API، شغل لوحة FastAPI
المحلية التي تتصل بنفس قاعدة PostgreSQL:

```powershell
cd .\services\cloud-api
python .\fastapi_app.py
```

ثم افتح `http://127.0.0.1:8080/`. راجع
[دليل لوحة المراقبة المحلية](local-dashboard.md) لإعداد `.env` ومسارات
الخدمة واحتياطات الأمان.

## قائمة تحقق للنشر

1. جهز `.env` للخادم داخل `services/cloud-api/` فقط.
2. اختبر اتصال PostgreSQL باستخدام `--init-db-only`.
3. انشر API خلف HTTPS.
4. نفذ `packaging/windows/package.cmd`.
5. جهز `.env` للعميل من ملف القالب داخل الحزمة.
   استبدل عنوان `activation.example.com` بعنوان API المنشور فعليا.
6. ثبت الخدمة من الواجهة بصلاحية Administrator.
7. ولد مفتاحا لكل نوع برنامج وتحقق من ظهوره في لوحة التحكم.
