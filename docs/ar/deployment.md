# البناء والنشر والتشغيل الإداري

## المعمارية المطلوبة للنشر

لا تنشر Cloud API للمولد. المطلوب هو:

| الموقع | ما يشغل فيه |
| --- | --- |
| خادم بعيد | PostgreSQL فقط |
| جهاز المولد | حزمة Rust وخدمة NSSM المحلية |
| جهاز المدير | لوحة FastAPI المحلية عند الحاجة |

خدمة Rust ولوحة FastAPI تستعملان اتصالات PostgreSQL مستقلة. لا تحتاج لوحة
FastAPI أن تكون مفتوحة كي يعمل مولد المفاتيح.

## متطلبات البناء

- Windows لبناء وتشغيل حزمة `exe` وخدمة NSSM.
- Rust/Cargo لتجميع الواجهة والخدمة.
- Python 3 و`pip` لتشغيل لوحة الإدارة المحلية واختبارها.
- PostgreSQL قابل للاتصال عبر SSL من أجهزة التشغيل المعتمدة.

## إعداد PostgreSQL

### الجداول

من جذر المشروع:

```powershell
cd .\services\cloud-api
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# حرر .env ببيانات حساب يستطيع إنشاء/تهيئة الجداول.
python .\app.py --init-db-only
```

ينشئ الأمر أو يحدث:

```text
server_control     حالة التشغيل الوحيدة id=1, status='0' أو '1'
activation_logs    سجلات المفاتيح المرفوعة من الأجهزة
```

تضاف قيمة البداية `server_control.status='1'` إن لم يكن صف التحكم موجودا.

### فصل حساب الإدارة عن حساب المولد

ملف `.env` الخاص بلوحة الإدارة يحتاج صلاحية قراءة السجلات وتعديل الحالة،
ويحتاج `DELETE` على `activation_logs` إذا سيستخدم زر حذف السجل.
أما ملف `.env` الذي يوضع مع حزمة المولد فيجب أن يستعمل حسابا محدودا، مثلا:

```sql
GRANT CONNECT ON DATABASE keygen_restaurant TO restricted_client_user;
GRANT USAGE ON SCHEMA public TO restricted_client_user;
GRANT SELECT ON TABLE server_control TO restricted_client_user;
GRANT INSERT ON TABLE activation_logs TO restricted_client_user;
GRANT USAGE, SELECT ON SEQUENCE activation_logs_id_seq TO restricted_client_user;
```

لا تمنح حساب المولد `UPDATE` على `server_control` ولا صلاحيات حذف السجلات.

## إعداد ملفات البيئة

### ملف المولد

أنشئ `packaging\windows\.env` للبناء المحلي، ثم ضع ملفا فعليا مماثلا بجانب
الحزمة على الجهاز الموثوق:

```dotenv
PGHOST=database-host
PGPORT=database-port
PGDATABASE=keygen_restaurant
PGUSER=restricted_client_user
PGPASSWORD=restricted_client_password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10
KEYGEN_LISTEN_ADDRESS=127.0.0.1:45632
KEYGEN_STATUS_INTERVAL_SECONDS=15
KEYGEN_UPLOAD_INTERVAL_SECONDS=5
KEYGEN_TOKEN_IDS=restaurant,lab,jewelry
KEYGEN_TOKEN_RESTAURANT_NAME=Restaurant
KEYGEN_TOKEN_RESTAURANT_SECRET=replace-with-private-restaurant-token
KEYGEN_TOKEN_LAB_NAME=Lab
KEYGEN_TOKEN_LAB_SECRET=replace-with-private-lab-token
KEYGEN_TOKEN_JEWELRY_NAME=Jewelry
KEYGEN_TOKEN_JEWELRY_SECRET=replace-with-private-jewelry-token
```

`KEYGEN_TOKEN_IDS` قائمة مرتبة قابلة للتوسعة. لكل معرف فيها يجب وجود
`KEYGEN_TOKEN_<ID>_NAME` للاسم الظاهر في الواجهة و
`KEYGEN_TOKEN_<ID>_SECRET` للسر المستخدم في اشتقاق المفتاح. راجع
[إعداد tokens السرية](tokens.md).

متغيرات اختيارية متقدمة:

| المتغير | الوظيفة |
| --- | --- |
| `KEYGEN_DATA_DIR` | تغيير مجلد ملفات البيانات المحلي |
| `KEYGEN_PRIMARY_LOG` | تغيير مسار سجل `generated_keys.txt` |

### ملف لوحة الإدارة

في `services\cloud-api\.env`:

```dotenv
PGHOST=database-host
PGPORT=database-port
PGDATABASE=keygen_restaurant
PGUSER=administrator_or_dashboard_user
PGPASSWORD=database-password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10
ADMIN_WEB_HOST=127.0.0.1
ADMIN_WEB_PORT=8080
```

## بناء حزمة Windows

من جذر المشروع:

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# حرر packaging\windows\.env دون مشاركته.
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

الناتج:

```text
dist\windows\ActivateurRMS\
|-- ActivateurRMS.exe
|-- SHA256SUMS.txt
|-- KeyGenService\KeyGenService.exe
`-- nssm\nssm.exe
```

أداة التغليف:

- تضمّن القيم المحلية التشغيلية غير السرية فقط في التنفيذيات.
- لا تضمّن تفاصيل اتصال PostgreSQL ولا أسماء أو أسرار tokens.
- تحذف أي `.env` أو `.env.example` قديم من مجلد الناتج.

بعد نقل الحزمة إلى الجهاز الذي تديره، ضع `.env` الحقيقي بجانب
`ActivateurRMS.exe`. لا تُسلّم نسخة تحتوي `.env` إلى جهاز غير موثوق.

## تثبيت أو تحديث الخدمة على جهاز المولد

```powershell
cd .\ActivateurRMS
.\ActivateurRMS.exe --install
```

السلوك:

1. يقرأ `.env` المحلي، بما فيه قائمة tokens وأسرار التوليد.
2. يطلب من backend المرفق تفويض التثبيت عبر PostgreSQL.
3. لا يتابع إلا إذا كانت الحالة `1`.
4. يطلب Administrator/UAC.
5. يوقف ويزيل أي خدمة قديمة بالاسم نفسه.
6. ينسخ الملفات إلى `%ProgramFiles%\KeyGenRMS`.
7. يسجل الخدمة كخدمة تلقائية ويبدأها.
8. يتأكد من أن backend يجيب محليا.

نفذ الأمر نفسه عند تحديث الحزمة؛ النسخ فقط لا يحدث الخدمة المثبتة. ونفذه
كذلك بعد إضافة token أو تغيير اسمه أو سرّه.

## إزالة الخدمة

```powershell
.\ActivateurRMS.exe --uninstall
```

أو:

```powershell
.\ActivateurRMS.exe --unstall
```

تحذف الإزالة خدمة Windows وملفات `%ProgramFiles%\KeyGenRMS`، لكنها لا
تحذف `%ProgramData%\KeyGenRMS`. افحص `pending_uploads.json` قبل حذف ملفات
البيانات يدويا، فقد يحتوي على سجلات لم ترفع بعد.

## تشغيل لوحة الإدارة

```powershell
cd .\services\cloud-api
python .\fastapi_app.py
```

افتح `http://127.0.0.1:8080/`. لتفاصيل التحكم والمسارات راجع
[لوحة FastAPI المحلية](local-dashboard.md).

## اختبار قبول النشر

1. اجعل الحالة `1` من لوحة الإدارة.
2. نفذ `ActivateurRMS.exe --install` على جهاز اختبار.
3. تحقق من تشغيل خدمة Windows باسم `KeyGenService`.
4. تحقق من إنشاء `%ProgramData%\KeyGenRMS\AUTHORIZED.txt`.
5. أنشئ مفتاحا وتأكد من ظهوره في لوحة الإدارة.
6. افصل الاتصال وأنشئ مفتاحا، ثم تحقق من وجوده في `pending_uploads.json`.
7. أعد الاتصال وتأكد من اختفاء العنصر المعلق وظهوره في PostgreSQL.
8. غيّر الحالة إلى `0` أثناء اتصال الجهاز وتأكد من رفض توليد جديد.
9. افصل الشبكة وتأكد أن الرفض يبقى فعالا بعد وصول `0`.
10. أعد الحالة إلى `1` وتأكد من عودة التوليد بعد اتصال الجهاز.
