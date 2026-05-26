# KeyGenRestorant / Activateur RMS

نظام إصدار مفاتيح تفعيل يتكون من:

- واجهة Windows خفيفة مكتوبة بـ Rust.
- خدمة محلية Rust لتوليد المفاتيح والمزامنة.
- Cloud API مكتوب بـ Python/Flask ويحفظ السجلات في PostgreSQL.
- لوحة مراقبة محلية بـ Python/FastAPI لعرض قاعدة البيانات والتحكم في الحالة.

## البدء السريع

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# حرر packaging\windows\.env بعنوان Cloud API المنشور وتوكن العميل.
cargo test
cargo clippy --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

ينتج التغليف حزمة العميل في:

```text
dist\windows\ActivateurRMS\
```

تضمّن عملية البناء إعدادات العميل المصفاة في ملفات Rust التنفيذية؛ لا
يُوزع ملف `.env` مع العميل ولا يجب أن يحتوي إعداد العميل بيانات PostgreSQL.

## هيكل المشروع

```text
apps/                 تطبيقات Rust التنفيذية
crates/               المكتبات المشتركة
services/cloud-api/   API السحابي وإعداد PostgreSQL ولوحة التحكم
packaging/windows/    مدخلات وسكربت تغليف عميل Windows
docs/                 أدلة الاستخدام والنشر ومرجع API
dist/                 نواتج محلية غير متتبعة في Git
```

## الوثائق

- [استخدام برنامج العميل](docs/ar/usage.md)
- [النشر والبناء والتشغيل](docs/ar/deployment.md)
- [تشغيل لوحة FastAPI المحلية](docs/ar/local-dashboard.md)
- [الأمان وإدارة الأسرار](docs/ar/security.md)
- [مرجع API](docs/api.md)
