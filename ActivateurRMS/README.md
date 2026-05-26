# Activateur RMS - دليل التشغيل والنشر

هذا المشروع نظام لإصدار مفاتيح تفعيل تطبيقات RMS. يتكون من واجهة مكتبية
وخدمة محلية مكتوبتين بلغة Rust، ومن API سحابي بسيط يتصل بقاعدة PostgreSQL.

البرامج المدعومة في الواجهة:

| الاختيار | القيمة البرمجية | الغرض |
| --- | --- | --- |
| تسيير المطاعم | `Restaurant` | مفتاح تطبيق المطاعم |
| تسيير مخزون المخابر | `Lab` | مفتاح تطبيق المخابر |
| تسيير محلات الذهب | `Jewelry` | مفتاح تطبيق المجوهرات |

كل اختيار يولد مفتاحا خاصا به لنفس `Request Code`. خوارزمية `Restaurant`
متوافقة مع الخوارزمية السابقة حتى لا تتغير المفاتيح المعروفة.

## البنية العامة

```text
Windows client package
|
|-- ActivateurRMS.exe           Rust GUI + installer/updater
|       |
|       | POST http://127.0.0.1:45632/generate_key (default)
|       v
|-- KeyGenService.exe           Rust local service installed by NSSM
|       |
|       | HTTPS + Bearer token
|       | GET  /api/v1/server-status
|       | POST /api/v1/activation-logs
|       v
Cloud API: server.py (Flask)
        |
        | SSL database connection
        v
PostgreSQL: keygen_restaurant
```

قاعدة PostgreSQL لا يتصل بها برنامج العميل مباشرة. بيانات اتصال قاعدة
البيانات تبقى على الخادم فقط، بينما يحتفظ العميل بعنوان API وتوكن الخدمة
المطلوبين للمزامنة والتحكم بحالة التوليد.

## ملفات المشروع

```text
ActivateurRMS/
|-- Cargo.toml                       Rust workspace
|-- Cargo.lock                       Locked Rust dependency versions
|-- ActivateurRust/                  Source of ActivateurRMS.exe
|-- KeyGenCommonRust/                Shared .env parser and app choices
|-- KeyGenServiceRust/               Source of KeyGenService.exe
|-- ActivateurRMS.exe                Packaged desktop program
|-- KeyGenService/KeyGenService.exe  Packaged local service
|-- nssm/nssm.exe                    Windows service wrapper
|-- client.env.example               Desktop .env template
|-- server.py                        Cloud API and dashboard
|-- server_requirements.txt          Python server dependencies
`-- .env.example                     Server .env template
```

## كيف يعمل النظام

### 1. تجهيز الخادم

يشغل المسؤول `server.py` على جهاز الخادم. عند البدء يتصل البرنامج بقاعدة
PostgreSQL باستخدام `.env` الموجود بجانبه، وينشئ الجدولين التاليين إن لم
يكونا موجودين:

| الجدول | وظيفته |
| --- | --- |
| `server_control` | الحالة العامة: `1` يسمح بالتوليد و`0` يوقفه للصيانة |
| `activation_logs` | سجلات المفاتيح المولدة والمرفوعة من العملاء |

### 2. تثبيت العميل

يقرأ `ActivateurRMS.exe` ملف `.env` المجاور له ويعرض URL الخادم والتوكن في
الواجهة. عند الضغط على `Installer / Mettre a jour`:

1. تتحقق الواجهة من التوكن باستدعاء `/api/v1/server-status`.
2. توقف خدمة `KeyGenService` القديمة إن كانت موجودة.
3. تنسخ `KeyGenService.exe` و`nssm.exe` إلى `%ProgramFiles%\KeyGenRMS`.
4. تكتب ملف `%ProgramFiles%\KeyGenRMS\.env` بإعدادات العميل فقط.
5. تثبت خدمة Windows باسم `KeyGenService` بواسطة NSSM وتبدأ تشغيلها تلقائيا.

يتطلب التثبيت أو التحديث صلاحية Administrator. يمكن توليد المفاتيح بعد ذلك
بدون تشغيل الواجهة بصلاحية Administrator.

### 3. توليد مفتاح

في الواجهة:

1. اختر البرنامج الفعلي: `Restaurant` أو `Lab` أو `Jewelry`.
2. أدخل `Request Code` بصيغة `XXXX-XXXX-XXXX`.
3. اضغط `Generer la cle`.

ترسل الواجهة الطلب إلى الخدمة المحلية. تتحقق الخدمة من عدم وجود وضع صيانة،
ثم تولد المفتاح المختص بنوع البرنامج، وتحفظ السجل محليا في قائمة انتظار.
تقوم الخدمة في الخلفية برفع السجلات المنتظرة إلى API السحابي.

### 4. التحكم والصيانة

تستعلم الخدمة دوريا عن حالة الخادم:

| الحالة | السلوك |
| --- | --- |
| `1` | التوليد مسموح ورفع السجلات يعمل |
| `0` | التوليد موقوف مؤقتا وتبقى البيانات محفوظة محليا |

يمكن للمسؤول تغيير الحالة ومراجعة السجلات من لوحة الويب:

```text
https://your-api-host/dashboard
```

تطلب اللوحة نفس `KEYGEN_API_SECRET_TOKEN` المستخدم لحماية API.

## متطلبات التشغيل

### جهاز العميل

- Windows 10 أو أحدث.
- صلاحية Administrator عند أول تثبيت أو عند تحديث الخدمة.
- الملفات الأربعة في حزمة واحدة: الواجهة، الخدمة، NSSM وملف `.env`.
- اتصال بخادم API عند التثبيت والمزامنة.

### خادم API

- Python 3.10 أو أحدث.
- PostgreSQL متاح عبر SSL.
- خدمة HTTPS أمام Flask في بيئة الإنتاج، مثل reverse proxy مناسب.

### جهاز البناء

- Rust toolchain حديث يدعم edition 2024.
- PowerShell على Windows لبناء وتغليف ملفات `exe`.

## إعداد ملف العميل `.env`

أنشئ ملف `.env` بجانب `ActivateurRMS.exe` انطلاقا من
[`client.env.example`](client.env.example):

```powershell
Copy-Item .\client.env.example .\.env
```

مثال:

```dotenv
KEYGEN_CLOUD_API_URL=https://activation.example.com
KEYGEN_API_SECRET_TOKEN=replace-with-api-token
KEYGEN_LISTEN_ADDRESS=127.0.0.1:45632
KEYGEN_STATUS_INTERVAL_SECONDS=300
KEYGEN_UPLOAD_INTERVAL_SECONDS=5
```

| المتغير | مطلوب | الوصف | الافتراضي |
| --- | --- | --- | --- |
| `KEYGEN_CLOUD_API_URL` | نعم للإنتاج | عنوان API السحابي بدون `/` نهائية | عنوان التطوير المضمن |
| `KEYGEN_API_SECRET_TOKEN` | نعم للمزامنة والتثبيت | Bearer token المشترك مع الخادم | فارغ |
| `KEYGEN_LISTEN_ADDRESS` | لا | عنوان الخدمة المحلية الذي تستخدمه الواجهة أيضا | `127.0.0.1:45632` |
| `KEYGEN_DATA_DIR` | لا | مجلد التخزين المحلي والانتظار | انظر قسم البيانات |
| `KEYGEN_PRIMARY_LOG` | لا | مسار سجل المفاتيح النصي الأساسي | انظر قسم البيانات |
| `KEYGEN_STATUS_INTERVAL_SECONDS` | لا | مدة فحص حالة الصيانة بالثواني | `300` |
| `KEYGEN_UPLOAD_INTERVAL_SECONDS` | لا | مدة محاولة رفع السجلات بالثواني | `5` |

ملاحظات مهمة:

- لا تضع `PGPASSWORD` أو أي إعداد PostgreSQL في ملف العميل.
- عند تعديل المنفذ المحلي في `KEYGEN_LISTEN_ADDRESS` أعد تثبيت/تحديث الخدمة
  بواسطة الواجهة حتى تستخدم الواجهة والخدمة القيمة نفسها.
- يمكن تحديد ملف إعداد مختلف للاختبار عبر متغير العملية
  `KEYGEN_ENV_FILE`. تتقدم متغيرات بيئة العملية على قيم `.env`.
- ملف `.env` إعداد تشغيل خارجي ولا يدمج داخل `exe`، حتى لا تثبت الأسرار
  داخل الملف التنفيذي ويمكن تدوير التوكن دون إعادة بناء البرنامج.

## تثبيت العميل واستخدامه

ضع الحزمة بالشكل التالي:

```text
Package\
|-- ActivateurRMS.exe
|-- .env
|-- KeyGenService\
|   `-- KeyGenService.exe
`-- nssm\
    `-- nssm.exe
```

خطوات التثبيت:

1. شغل `ActivateurRMS.exe`.
2. تأكد من عنوان `Serveur Cloud API` والتوكن المعروضين.
3. اضغط `Relancer comme administrateur` إن لم تكن الواجهة مرفوعة الصلاحية.
4. اضغط `Installer / Mettre a jour`.
5. بعد ظهور رسالة نجاح التثبيت، اختر البرنامج وأدخل `Request Code`.
6. اضغط `Generer la cle`؛ يظهر المفتاح في خانة `Cle d'activation`.

موقع الخدمة المثبتة:

```text
%ProgramFiles%\KeyGenRMS\
|-- KeyGenService.exe
|-- nssm.exe
`-- .env
```

## إعداد وتشغيل الخادم

ثبت اعتماديات Python:

```powershell
cd .\ActivateurRMS
python -m pip install -r .\server_requirements.txt
```

أنشئ ملف إعداد الخادم من [`.env.example`](.env.example):

```powershell
Copy-Item .\.env.example .\.env
```

مثال إعداد الخادم:

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

| المتغير | مطلوب | الوصف |
| --- | --- | --- |
| `PGHOST` | نعم ما لم يستخدم `DATABASE_URL` | خادم PostgreSQL |
| `PGPORT` | نعم ما لم يستخدم `DATABASE_URL` | منفذ PostgreSQL |
| `PGDATABASE` | نعم ما لم يستخدم `DATABASE_URL` | اسم قاعدة البيانات |
| `PGUSER` | نعم ما لم يستخدم `DATABASE_URL` | مستخدم قاعدة البيانات |
| `PGPASSWORD` | نعم ما لم يستخدم `DATABASE_URL` | كلمة مرور قاعدة البيانات |
| `PGSSLMODE` | موصى به | وضع SSL، الافتراضي `require` |
| `PGCONNECT_TIMEOUT` | لا | مهلة الاتصال بالثواني، الافتراضي `10` |
| `DATABASE_URL` | بديل | DSN كامل بدلا من متغيرات `PG*` |
| `KEYGEN_API_SECRET_TOKEN` | نعم | توكن API المطابق لملفات العملاء |
| `FLASK_DEBUG` | لا | اتركه `0` في الإنتاج |

شغل API:

```powershell
python .\server.py
```

يستمع تشغيل Flask المباشر على:

```text
http://0.0.0.0:7002
```

في الإنتاج لا تعرض هذا الاتصال عبر HTTP العام؛ ضع API خلف HTTPS ثم استخدم
عنوان HTTPS في ملف العميل.

## ترحيل بيانات SQLite القديمة

إذا كانت لديك قاعدة SQLite سابقة، نفذ الاستيراد مرة واحدة على الخادم بعد
إعداد `.env`:

```powershell
python .\server.py --migrate-sqlite .\cloud_database.db
```

الاستيراد idempotent بالنسبة إلى معرفات سجلات التفعيل: إعادة تشغيله لا
تكرر السجلات ذات المعرف الموجود.

## مسارات التخزين المحلية

عند عدم تحديد مسارات مخصصة على Windows تستخدم الخدمة:

| البيانات | المسار |
| --- | --- |
| السجل الأساسي للمفاتيح | `C:\key_storage\generated_keys.txt` |
| قائمة انتظار الرفع | `C:\ProgramData\SystemLogs\pending_uploads.json` |
| سجل الشبكة المحلي | `C:\ProgramData\SystemLogs\netcache.dat` |
| سجل العناصر المرفوعة | `C:\ProgramData\SystemLogs\uploaded.log` |
| مؤشر وضع الصيانة | `C:\ProgramData\SystemLogs\MAINTENANCE.txt` |

يمكن تغيير مجلد البيانات عبر `KEYGEN_DATA_DIR` ومسار السجل الأساسي عبر
`KEYGEN_PRIMARY_LOG` في ملف العميل، ثم إعادة تحديث الخدمة.

## مرجع API

### المصادقة السحابية

كل المسارات `/api/v1/*` تتطلب:

```http
Authorization: Bearer <KEYGEN_API_SECRET_TOKEN>
Content-Type: application/json
```

### `GET /api/v1/server-status`

يعيد حالة التوليد:

```json
{"status": "1"}
```

### `POST /api/v1/set-status`

يغير حالة التوليد:

```json
{"status": "0"}
```

القيم المقبولة هي `1` للتشغيل و`0` للصيانة.

### `GET /api/v1/activation-logs`

يعيد سجلات التفعيل المخزنة في PostgreSQL مرتبة من الأحدث إلى الأقدم.

### `POST /api/v1/activation-logs`

تستعمله خدمة Rust لرفع قائمة سجلات:

```json
[
  {
    "request_code": "F81A-67A7-C6AA",
    "activation_key": "EE8C-551F-0A90-73F5",
    "generated_at": "2026-05-26T16:00:00.000000+00:00",
    "device_ip": "127.0.0.1"
  }
]
```

### `GET /health` على الخدمة المحلية

عنوانه الافتراضي:

```text
http://127.0.0.1:45632/health
```

مثال استجابة:

```json
{"status":"ok","maintenance":false,"cloud_api_url":"https://activation.example.com"}
```

### `POST /generate_key` على الخدمة المحلية

طلب:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "app_type": "Restaurant",
  "server_url": "https://activation.example.com"
}
```

استجابة نجاح:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "activation_key": "EE8C-551F-0A90-73F5",
  "app_type": "Restaurant",
  "status": "generated_and_queued"
}
```

إذا كانت حالة الخادم صيانة، يعيد المسار HTTP `503` ولا يولد مفتاحا.

## بناء البرنامج

من جذر المستودع:

```powershell
cd .\ActivateurRMS
cargo fmt --all
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
Copy-Item .\target\release\ActivateurRMS.exe .\ActivateurRMS.exe -Force
Copy-Item .\target\release\KeyGenService.exe .\KeyGenService\KeyGenService.exe -Force
```

يستخدم ملف workspace إعدادات release موجهة إلى تقليل الحجم:

- `opt-level = "z"`
- `lto = true`
- `strip = "symbols"`
- `panic = "abort"`
- `codegen-units = 1`

## التحقق قبل النشر

1. تأكد من أن `.env` الخاص بالخادم لا يوجد داخل حزمة العميل.
2. تأكد من أن `client .env` يحتوي عنوان HTTPS وتوكن API الصحيح فقط.
3. اختبر `/api/v1/server-status` من الشبكة التي سيعمل عليها العميل.
4. ثبت الخدمة من الواجهة بصلاحيات Administrator.
5. ولد مفتاح اختبار لكل برنامج من البرامج الثلاثة.
6. تحقق من ظهور السجلات في `/dashboard` أو `/api/v1/activation-logs`.
7. اختبر الحالة `0` ثم أعدها إلى `1` بعد التأكد من تعطيل التوليد.

## استكشاف الأعطال

| المشكلة | السبب المحتمل | الإجراء |
| --- | --- | --- |
| فشل التثبيت مع خطأ API | URL أو التوكن غير صحيح، أو API غير متاح | راجع `.env` واختبر عنوان HTTPS والتوكن |
| الواجهة لا تستطيع التوليد | الخدمة غير مثبتة أو المنفذ مختلف | أعد التثبيت، وتأكد من تطابق `KEYGEN_LISTEN_ADDRESS` |
| رسالة صيانة عند التوليد | حالة `server_control` تساوي `0` | غير الحالة من لوحة التحكم أو API إلى `1` |
| السجلات لا تظهر في الخادم | توكن العميل خاطئ أو الاتصال منقطع | راجع توكن العميل؛ السجلات تبقى في `pending_uploads.json` للمحاولة لاحقا |
| فشل الخادم عند البدء | إعداد PostgreSQL ناقص أو كلمة المرور غير صحيحة | راجع `.env` الخاص بالخادم واتصال PostgreSQL/SSL |
| ملفات البيانات في مكان غير مرغوب | المسارات الافتراضية مستخدمة | اضبط `KEYGEN_DATA_DIR` و`KEYGEN_PRIMARY_LOG` ثم حدث الخدمة |

## الأمان

- لا تلتزم بملفات `.env` الفعلية في Git.
- لا توزع بيانات PostgreSQL مع `ActivateurRMS.exe` أو `KeyGenService.exe`.
- استخدم HTTPS لاتصال العميل بالخادم؛ التوكن وسجلات التفعيل يمران عبر API.
- دوّر `KEYGEN_API_SECRET_TOKEN` عند تسريب حزمة عميل أو خروج جهاز من الثقة.
- اجعل الوصول إلى قاعدة PostgreSQL مقتصرا على خادم API قدر الإمكان.
