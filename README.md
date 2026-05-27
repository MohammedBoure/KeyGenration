# KeyGenRestorant / Activateur RMS

نظام Windows لتوليد مفاتيح التفعيل ومتابعتها، يتكون من:

- واجهة رسومية خفيفة مكتوبة بـ Rust: `ActivateurRMS.exe`.
- خدمة خلفية Rust مثبتة عبر NSSM: `KeyGenService.exe`.
- قاعدة PostgreSQL خارجية لحالة التشغيل وسجلات المفاتيح فقط.
- لوحة FastAPI محلية لعرض السجلات وتغيير الحالة بين `1` و`0`.

## منطق التشغيل

1. تفتح واجهة البرنامج وتتحقق من تسجيل خدمة Windows `KeyGenService` التي يديرها NSSM ومن backend المحلي.
2. إن كانت الخدمة الجديدة تعمل، تستمر الواجهة مباشرة حتى دون إنترنت.
3. إن كانت غير مثبتة أو قديمة، يتطلب التثبيت اتصالا بـ PostgreSQL وكون
   `server_control.status = '1'` قبل طلب صلاحية التثبيت، ثم تثبت الواجهة الخدمة بواسطة NSSM.
4. عند بدء الخدمة لأول مرة، تعيد تأكيد الحالة `1` وتكتب `AUTHORIZED.txt` محليا؛
   لا تعمل نسخة جديدة دون اتصال قبل هذه الخطوة.
5. تولد الخدمة المفاتيح محليا، وتحفظ كل عملية محليا في مجلد بيانات النظام.
6. ترفع الخدمة السجلات المعلقة إلى PostgreSQL عند توفر الاتصال.
7. عند توفر الاتصال، تقرأ الخدمة الحالة. إذا أصبحت `0` تحفظ وضع الإيقاف
   محليا وترفض توليد مفاتيح جديدة حتى تقرأ `1` لاحقا.

عند انقطاع الإنترنت تستمر الخدمة وفق آخر حالة عرفتها. لذلك، إذا انقطع
الاتصال قبل وصول قيمة `0` إلى الجهاز، لا يمكن إيقاف التوليد فوريا.

## إدارة الخدمة من CMD

```powershell
.\ActivateurRMS.exe --install
.\ActivateurRMS.exe --uninstall
```

يدعم البرنامج أيضا `--unstall` كاسم بديل للإزالة. يطلب الأمران صلاحية
Administrator عند الحاجة، وتحافظ الإزالة على بيانات `%ProgramData%\KeyGenRMS`.

## البدء السريع

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# استعمل القالب لبناء الحزمة، وضع ملف .env فعليا بجانب exe على جهاز التشغيل فقط.
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

ينتج التغليف الحزمة في `dist\windows\ActivateurRMS\`.

## هيكل المشروع

```text
apps/desktop/          واجهة Windows ومثبت خدمة NSSM
apps/keygen-service/   خدمة التوليد المحلي والمزامنة المؤجلة
crates/config/         قراءة إعدادات .env المضمنة والمحلية
services/cloud-api/    تهيئة PostgreSQL ولوحة FastAPI المحلية (اسم تاريخي)
packaging/windows/     تجميع حزمة Windows
docs/                  أدلة الاستخدام والأمان والواجهات
```

## الأمان

لأن الخدمة تتصل مباشرة بقاعدة البيانات وتولد دون إنترنت، فإن الحزمة تحتوي
خوارزمية التوليد. لا تضمّن كلمة مرور PostgreSQL في `exe`: توضع في ملف `.env`
محلي بجانب البرنامج على الجهاز الذي تديره، ويفضل أن تخص حسابا محدودا
بصلاحية قراءة الحالة وإدخال السجلات فقط.

## الوثائق

- [الاستخدام](docs/ar/usage.md)
- [البناء والتشغيل](docs/ar/deployment.md)
- [لوحة FastAPI المحلية](docs/ar/local-dashboard.md)
- [الأمان](docs/ar/security.md)
- [مرجع الواجهات والتخزين](docs/api.md)
