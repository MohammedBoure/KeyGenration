# استكشاف الأخطاء والصيانة

## أوامر فحص أساسية

نفذ هذه الأوامر في PowerShell على جهاز المولد:

```powershell
Get-Service -Name KeyGenService
sc.exe qc KeyGenService
Invoke-RestMethod http://127.0.0.1:45632/health
Get-Content "$env:ProgramData\KeyGenRMS\pending_uploads.json"
```

وعلى جهاز الإدارة:

```powershell
Invoke-RestMethod http://127.0.0.1:8080/api/status
Invoke-RestMethod "http://127.0.0.1:8080/api/activation-logs?limit=100"
```

## `Demarrage du service impossible`

### السبب الأكثر شيوعا

توجد خدمة Windows قديمة تشير إلى `nssm.exe` أو `KeyGenService.exe` لم يعد
موجودا. يمكن كشف ذلك عبر:

```powershell
sc.exe qc KeyGenService
```

### الإصلاح

من مجلد الحزمة الجديدة الذي يحتوي `.env`:

```powershell
.\ActivateurRMS.exe --install
```

وافق على UAC. الأمر يفحص PostgreSQL، ويستبدل الخدمة القديمة أو المكسورة
بالنسخة المرفقة، ثم يبدأ backend الجديد.

## التثبيت لا يكتمل

| العرض | السبب المحتمل | الإجراء |
| --- | --- | --- |
| رفض التثبيت بسبب التعطيل | `server_control.status=0` | غيّر الحالة إلى `1` من لوحة الإدارة |
| خطأ اتصال PostgreSQL | إنترنت غير متاح أو `.env` خاطئ | راجع `PGHOST` و`PGPORT` وSSL وكلمة المرور |
| رفض UAC | لم تمنح صلاحية Administrator | أعد تشغيل `--install` ووافق على UAC |
| Backend مفقود أو NSSM مفقود | الحزمة ناقصة | أعد نسخ الحزمة كاملة دون فصل المجلدات |
| `Configuration des tokens invalide` | إعداد token ناقص أو متكرر | راجع `KEYGEN_TOKEN_IDS` وحقلي `NAME` و`SECRET` لكل معرف |

## الواجهة تفتح لكن التوليد لا يعمل

1. تحقق من الخدمة:

```powershell
Get-Service KeyGenService
Invoke-RestMethod http://127.0.0.1:45632/health
```

2. إذا كان الرد يحتوي:

```json
{"maintenance":true}
```

فقد استلمت الخدمة الحالة `0`. غيّرها إلى `1` من لوحة الإدارة واترك الجهاز
متصلا حتى تستلم الخدمة التغيير.

3. إذا لم يرد المنفذ المحلي، نفذ:

```powershell
.\ActivateurRMS.exe --install
```

## `pending_uploads` لا يصبح صفرا

في استجابة `/health`:

```json
{"pending_uploads":6}
```

تعني أن مفاتيح أنشئت محليا ولم تؤكد الخدمة إزالة سجلاتها من الطابور بعد.

افحص بالترتيب:

1. اتصال الجهاز بـ PostgreSQL.
2. صلاحية الحساب على `activation_logs`.
3. أن الخدمة المثبتة هي آخر نسخة من الحزمة:

```powershell
.\ActivateurRMS.exe --install
```

4. أن السجلات ظهرت في لوحة FastAPI.

لا تحذف `pending_uploads.json` لمجرد أن الطابور ظاهر؛ قد تفقد سجلات لم تصل
بعد. إذا وصلت السجلات فعليا وبقي الطابور بسبب إصدار قديم، حدّث الخدمة
بالأمر `--install` كي يؤكد الرفع وينظفه.

## فرغت PostgreSQL ثم ظهرت سجلات مجددا

سبب ذلك عادة أن جهاز مولد كان يحمل سجلات في `pending_uploads.json`، ورفعها
بعد عودة الاتصال. قبل تفريغ قاعدة البيانات:

1. أوقف التوليد أو اجعل الحالة `0` واترك الأجهزة تتصل لتستلمها.
2. تحقق من طوابير الأجهزة.
3. اسمح برفع المطلوب أو احتفظ بنسخة منه.
4. فرغ `activation_logs` فقط، مع إبقاء `server_control`.

## موقع FastAPI لا يبدأ أو لا يعرض البيانات

### خطأ إعدادات PostgreSQL المطلوبة في `.env`

أنشئ الملف:

```powershell
cd .\services\cloud-api
Copy-Item .\.env.example .\.env
```

ثم ضع بيانات الاتصال الفعلية لكل من `PGHOST` و`PGPORT` و`PGDATABASE`
و`PGUSER` و`PGPASSWORD` في `.env`.

### الموقع يعمل لكن الطلبات من جهاز آخر مرفوضة

هذا هو السلوك الآمن الافتراضي؛ الموقع محلي فقط. لا تفعّل الوصول البعيد إلا
إذا أضفت المصادقة والحماية المناسبة.

## التحديث والإزالة

تحديث الخدمة:

```powershell
.\ActivateurRMS.exe --install
```

إزالتها:

```powershell
.\ActivateurRMS.exe --uninstall
```

الإزالة لا تمسح `%ProgramData%\KeyGenRMS`. بعد التأكد من عدم وجود سجلات
معلقة يمكنك أرشفة المجلد أو حذفه بقرار مستقل.
