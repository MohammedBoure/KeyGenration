# البناء والتشغيل

لا توجد خدمة Cloud API عامة في هذا التصميم. خدمة Rust المثبتة تتصل مباشرة
بـ PostgreSQL، ولوحة FastAPI مخصصة للتشغيل المحلي على جهاز الإدارة.

## إعداد PostgreSQL ولوحة الإدارة المحلية

من مجلد `services\cloud-api`، وهو اسم تاريخي للمجلد:

```powershell
Copy-Item .\.env.example .\.env
# ضع بيانات PostgreSQL الفعلية في .env
python .\app.py --init-db-only
python .\fastapi_app.py
```

ثم افتح:

```text
http://127.0.0.1:8080/
```

تقرأ اللوحة سجلات `activation_logs` وتغير قيمة `server_control.status`
بين `1` و`0`.

## حساب البرنامج الموزع

تقرأ خدمة Rust بيانات الاتصال من ملف `.env` المحلي في جهاز التشغيل؛ ولا
يضمّن البناء اسم المستخدم أو كلمة المرور في `exe`. مع ذلك لا تستخدم الحساب
الإداري أو حساب مالك القاعدة على جهاز غير موثوق. الحساب المحدود يحتاج فقط:

```sql
GRANT CONNECT ON DATABASE keygen_restaurant TO restricted_client_user;
GRANT USAGE ON SCHEMA public TO restricted_client_user;
GRANT SELECT ON TABLE server_control TO restricted_client_user;
GRANT INSERT ON TABLE activation_logs TO restricted_client_user;
GRANT USAGE, SELECT ON SEQUENCE activation_logs_id_seq TO restricted_client_user;
```

يجب ألا يمتلك هذا الحساب صلاحية `UPDATE` على `server_control`.

## بناء حزمة Windows

من جذر المشروع:

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
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

لا تضمّن الحزمة `PGUSER` أو `PGPASSWORD`. على جهاز التشغيل الذي تديره، انسخ
القالب كملف `.env` بجانب `ActivateurRMS.exe` وضع بيانات PostgreSQL فيه. بعد
التثبيت تحفظ الخدمة إعدادها المحلي داخل `%ProgramFiles%\KeyGenRMS\.env`.

## اختبار دورة التشغيل

1. اجعل الحالة `1` من لوحة الإدارة.
2. شغل الواجهة على جهاز اختبار واتركها تثبت خدمة NSSM.
3. أنشئ مفتاحا وتحقق من ظهوره في اللوحة بعد المزامنة.
4. افصل الاتصال وأنشئ مفتاحا؛ تحقق من وجوده في `pending_uploads.json`.
5. أعد الاتصال وتحقق من رفع السجل.
6. غيّر الحالة إلى `0` أثناء اتصال الجهاز؛ بعد وصولها للخدمة يجب أن ترفض
   المفاتيح الجديدة.
7. أعد الحالة إلى `1` وتحقق من عودة التوليد.
