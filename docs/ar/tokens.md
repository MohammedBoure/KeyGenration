# إعداد tokens السرية

## الهدف

قائمة البرامج وأسرار اشتقاق مفاتيحها ليست ثابتة في كود Rust. مصدرها الوحيد
على جهاز المولد هو ملف `.env` المحلي بجانب `ActivateurRMS.exe`، ثم تنسخ
الواجهة القيم المطلوبة إلى `%ProgramFiles%\KeyGenRMS\.env` عند تثبيت الخدمة.

لا تضف ملف `.env` الفعلي إلى Git ولا تضع الأسرار في `client.env.example`.

## الصيغة

```dotenv
KEYGEN_TOKEN_IDS=restaurant,lab,jewelry

KEYGEN_TOKEN_RESTAURANT_NAME=Restaurant
KEYGEN_TOKEN_RESTAURANT_SECRET=replace-with-private-restaurant-token

KEYGEN_TOKEN_LAB_NAME=Lab
KEYGEN_TOKEN_LAB_SECRET=replace-with-private-lab-token

KEYGEN_TOKEN_JEWELRY_NAME=Jewelry
KEYGEN_TOKEN_JEWELRY_SECRET=replace-with-private-jewelry-token
```

| المتغير | الوظيفة |
| --- | --- |
| `KEYGEN_TOKEN_IDS` | ترتيب الخيارات الظاهرة في الواجهة، مفصولة بفواصل |
| `KEYGEN_TOKEN_<ID>_NAME` | الاسم الظاهر للمستخدم والممرر إلى backend |
| `KEYGEN_TOKEN_<ID>_SECRET` | السر الذي تستخدمه خدمة Rust لاشتقاق المفتاح |

المعرف `<ID>` يقبل الحروف والأرقام و`_`، ويعامل دون حساسية لحالة الأحرف؛
مثلا `new_product` في القائمة يستخدم المتغيرين
`KEYGEN_TOKEN_NEW_PRODUCT_NAME` و`KEYGEN_TOKEN_NEW_PRODUCT_SECRET`.

يجب أن يكون كل اسم فريدا وألا تكون أي قيمة `SECRET` فارغة.

## إضافة token جديد

مثال إضافة برنامج جديد:

```dotenv
KEYGEN_TOKEN_IDS=restaurant,lab,jewelry,inventory
KEYGEN_TOKEN_INVENTORY_NAME=Inventory
KEYGEN_TOKEN_INVENTORY_SECRET=replace-with-new-private-token
```

بعد تحرير `.env` على جهاز المولد:

```powershell
.\ActivateurRMS.exe --install
```

ينسخ الأمر القائمة الجديدة وأسرارها إلى خدمة Windows ويعيد تشغيلها. كما
تفحص الواجهة أسماء tokens من `/health`؛ إذا كانت الخدمة المثبتة تحمل قائمة
قديمة، تعاملها كخدمة تحتاج إلى تحديث.

## تغيير سر token

تغيير قيمة `SECRET` يغير المفاتيح الناتجة لنفس كود الطلب مستقبلا. لذلك:

1. احتفظ بالسر السابق خارج المستودع إذا كنت تحتاج إعادة إنتاج مفاتيح قديمة.
2. عدل القيمة في `.env` الخاص بالأجهزة المعتمدة فقط.
3. نفذ `ActivateurRMS.exe --install` لتحديث الخدمة.
4. تحقق من توليد مفتاح اختبار ورفع سجله.

## حدود الأمان

- خدمة Rust تحتاج السر محليا لأنها تولد مفاتيح أثناء عدم توفر الإنترنت.
- عدم تضمين السر في `exe` يمنع نشره العرضي في المستودع أو ملف تنفيذي عام،
  لكنه لا يمنع مدير جهاز المولد من قراءة ملف الخدمة المحلي.
- أي سر كان موجودا في تاريخ Git سابقا يجب اعتباره مكشوفا وتدويره قبل جعل
  المستودع عاما.
