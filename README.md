# KeyGenRestorant / Activateur RMS

برنامج Windows خفيف لإنتاج مفاتيح تفعيل ثلاثة منتجات (`Restaurant` و`Lab`
و`Jewelry`) مع تسجيل العمليات في PostgreSQL والتحكم في إيقاف المولد عن بعد.
الواجهة والخدمة الخلفية مكتوبتان بـ Rust، أما موقع الإدارة فهو تطبيق FastAPI
محلي على جهاز المدير.

## الصورة العامة

```text
جهاز المولد
  ActivateurRMS.exe (واجهة Rust)
       |
       | HTTP محلي فقط: 127.0.0.1:45632
       v
  KeyGenService.exe (خدمة Rust مثبتة عبر NSSM)
       |
       | PostgreSQL + SSL
       v
  قاعدة PostgreSQL البعيدة
       ^
       | PostgreSQL + SSL
       |
جهاز الإدارة
  FastAPI المحلي: http://127.0.0.1:8080/
```

لا توجد Cloud API عامة بين المولد وقاعدة البيانات. خدمة Rust تتصل مباشرة
بـ PostgreSQL لقراءة حالة السماح ورفع السجلات، ولوحة FastAPI تتصل مباشرة
بنفس القاعدة لعرض البيانات وتعديل الحالة.

## المكونات

| المكون | الوظيفة |
| --- | --- |
| `ActivateurRMS.exe` | واجهة المولد، وفحص/تثبيت/تحديث/حذف خدمة Windows |
| `KeyGenService.exe` | توليد المفتاح محليا، حفظ الطابور، المزامنة، وتنفيذ حالة الإيقاف |
| `nssm.exe` | تشغيل خدمة Rust تلقائيا كخدمة Windows باسم `KeyGenService` |
| PostgreSQL | تخزين `server_control.status` وسجلات `activation_logs` |
| FastAPI المحلي | موقع الإدارة المحلي لعرض السجلات وحذف سجل عند الحاجة وتغيير `0/1` |

## مسار التشغيل

1. تشغل الواجهة وتفحص خدمة Windows المسجلة باسم `KeyGenService`.
2. إن كانت الخدمة تعمل وتجيب بصيغة backend الصحيحة، تظهر الواجهة جاهزة.
3. إن كانت الخدمة متوقفة، تحاول الواجهة بدءها.
4. إن لم تكن مثبتة، أو كانت قديمة/مكسورة، تفحص الواجهة اتصال PostgreSQL
   وتشترط أن تكون قيمة `server_control.status` مساوية لـ `1`.
5. بعد نجاح الفحص تطلب صلاحية Administrator، وتثبت أو تصلح الخدمة عبر NSSM.
6. في أول تشغيل للخدمة، تعيد الخدمة قراءة الحالة `1` ثم تكتب
   `%ProgramData%\KeyGenRMS\AUTHORIZED.txt`.
7. عند التوليد، تنشئ الخدمة المفتاح محليا وتحفظ السجل في طابور محلي، ثم
   ترفعه إلى PostgreSQL عند توفر الاتصال.
8. تقرأ الخدمة حالة `0/1` دوريا. وصول `0` ينشئ `MAINTENANCE.txt` ويمنع
   توليد مفاتيح جديدة، ووصول `1` يعيد التمكين.

## التشغيل السريع

### مولد Windows

الحزمة المجمعة تكون بالشكل التالي:

```text
ActivateurRMS\
|-- ActivateurRMS.exe
|-- KeyGenService\KeyGenService.exe
|-- nssm\nssm.exe
`-- SHA256SUMS.txt
```

على جهاز التشغيل الذي تديره فقط، ضع ملف `.env` مهيأ بجانب
`ActivateurRMS.exe`، ثم ثبت الخدمة:

```powershell
cd .\dist\windows\ActivateurRMS
.\ActivateurRMS.exe --install
.\ActivateurRMS.exe
```

سيظهر طلب UAC عند حاجة البرنامج إلى صلاحية تثبيت خدمة Windows.

### موقع الإدارة المحلي

```powershell
cd .\services\cloud-api
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# حرر .env ببيانات PostgreSQL الإدارية.
python .\app.py --init-db-only
python .\fastapi_app.py
```

ثم افتح `http://127.0.0.1:8080/`.

## أوامر الخدمة

يجب تنفيذ الأوامر من مجلد الحزمة الذي يحتوي `KeyGenService\` و`nssm\`:

| الأمر | النتيجة |
| --- | --- |
| `.\ActivateurRMS.exe --install` | تثبيت الخدمة أو تحديثها أو إصلاح تسجيل مكسور |
| `.\ActivateurRMS.exe --uninstall` | إيقاف وحذف خدمة backend وملفات التثبيت |
| `.\ActivateurRMS.exe --unstall` | اسم بديل مقبول لأمر الإزالة |
| `.\ActivateurRMS.exe --help` | عرض الأوامر |

`--uninstall` يحذف `%ProgramFiles%\KeyGenRMS`، لكنه يبقي
`%ProgramData%\KeyGenRMS` حتى لا تضيع مفاتيح أو سجلات غير مرفوعة.

## ملفات التشغيل

| المكان | المحتوى |
| --- | --- |
| بجانب `ActivateurRMS.exe` | ملف `.env` الأولي على جهاز التشغيل |
| `%ProgramFiles%\KeyGenRMS` | الخدمة المثبتة وNSSM ونسخة إعداد الخدمة |
| `%ProgramData%\KeyGenRMS` | سجل المفاتيح والطابور وحالة التفويض/الإيقاف |
| `services\cloud-api\.env` | إعداد لوحة FastAPI المحلية على جهاز الإدارة |

ملفات البيانات المحلية الأساسية:

| الملف | المعنى |
| --- | --- |
| `generated_keys.txt` | السجل المحلي للمفاتيح المولدة |
| `pending_uploads.json` | سجلات تنتظر الرفع إلى PostgreSQL |
| `uploaded.log` | سجلات رفعت بنجاح |
| `AUTHORIZED.txt` | تفويض التشغيل الأول بعد استلام الحالة `1` |
| `MAINTENANCE.txt` | إيقاف محلي بعد استلام الحالة `0` |

## البناء

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# اضبط القيم غير السرية المطلوبة للبناء.
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

الناتج الافتراضي هو `dist\windows\ActivateurRMS\`. سكربت التغليف لا يضمن
`PGUSER` أو `PGPASSWORD` في ملفات `exe`، ويحذف أي `.env` قديم من مجلد
الناتج. يوضع `.env` الحقيقي لاحقا على جهاز التشغيل فقط.

## الأمان والحدود

- استخدم حساب PostgreSQL محدودا للمولد: قراءة `server_control` وإدخال
  `activation_logs` فقط.
- لا توزع ملف `.env` الذي يحتوي كلمة المرور.
- لأن المولد يعمل دون إنترنت، خوارزمية التوليد موجودة على جهاز المولد.
- إذا انقطع الجهاز قبل استلام الحالة `0`، فلن يعرف الإيقاف الجديد حتى
  يتصل مرة أخرى. هذا حد ضروري للعمل offline.
- موقع FastAPI محلي افتراضيا ولا ينبغي نشره للإنترنت دون مصادقة وحماية.

## التوثيق التفصيلي

| الدليل | المحتوى |
| --- | --- |
| [فهرس التوثيق](docs/README.md) | نقطة البداية لكل الأدلة |
| [المعمارية وتدفق البيانات](docs/ar/architecture.md) | الحدود بين الواجهة والخدمة والقاعدة والموقع |
| [دليل الاستخدام](docs/ar/usage.md) | تشغيل المولد والأوامر والملفات المحلية |
| [البناء والنشر](docs/ar/deployment.md) | PostgreSQL والتجميع والتثبيت والتحقق |
| [لوحة FastAPI المحلية](docs/ar/local-dashboard.md) | تشغيل الموقع ومساراته والتحكم |
| [استكشاف الأخطاء](docs/ar/troubleshooting.md) | مشاكل الخدمة والمزامنة والاتصال |
| [الأمان وحدود التحكم](docs/ar/security.md) | الأسرار والصلاحيات والعمل دون إنترنت |
| [مرجع الواجهات والتخزين](docs/api.md) | HTTP وSQL والإعدادات للمطور |
