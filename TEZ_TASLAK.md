# Dağıtık Sistemlerde Kök Neden Analizi için Çok Katmanlı Bir Araç Zinciri

### Mimari Tasarım, Matematiksel Tanımlamalar ve Deneysel Değerlendirme

**Tür:** Yüksek Lisans Tezi (Taslak Metin)
**Kurum:** Mimar Sinan Güzel Sanatlar Üniversitesi — Fen Bilimleri Enstitüsü
**Dayandığı yazım kılavuzu:** MSGSÜ Tez/Eser Metni Yazım Kılavuzu, 2017
**Tarih:** 11 Ekim 2026

---

## OKUYAN İÇİN KISA NOT

Bu metin üç tür şey içerir ve aralarındaki ayrım önemlidir:

| Bölüm | Türü | Nasıl okunmalı |
|---|---|---|
| 2. ve 3. bölümlerin kavramsal kısımları | **Tanım** | Ezberlenmemeli, kavranmalı |
| 3.3 ve 3.4 | **Matematik** | Semboller tanımlıdır; sonuçlar örneklenmiştir |
| 3.7 | **Süreç** | Bunlar hata değil, sürecin öğrenilmiş aşamalarıdır |

4. bölümdeki her sayı gerçek bir ölçümdür ve EK-2'deki komutlarla yeniden üretilebilir.

---

## ETİK BEYAN VE ARAŞTIRMA SÜRECİNDE YAPAY ZEKÂ KULLANIMI

MSGSÜ kılavuzu (3.1) bilimsel araştırma etiğini, "yazarın araştırmasında hangi bilgiye
nasıl ulaşıldığını dürüstçe ve açık olarak belirtmesi" olarak tanımlar. Bu çalışmada
üretken yapay zekâ aracından yararlanılmıştır.

**Kullanılan araç:** Claude (Anthropic, `claude-opus-4-8`, 1M bağlam penceresi),
geliştirici ortamı OpenCode. Yardımcı sunucular: `paper` (arXiv araştırma asistanı),
`checker` (tez ön kontrolü). Çalışma dönemi: Ekim 2026.

**YÖK Etik Rehberi'ne (Mayıs 2024) göre kullanılan ve kullanılmayan alanlar:**

| Alan | Rehberin niteliği | Bu çalışmada |
|---|---|---|
| Kaynak araştırması, ön eleme | İzinli | Kullanıldı |
| Dil bilgisi ve yazım denetimi | İzinli | Kullanıldı |
| Biçim denetimi (kılavuza uygunluk) | İzinli | Kullanıldı |
| Hipotez ve araştırma sorusu | **Yasak** | Kullanılmadı — yazar tarafından belirlendi |
| Tartışma, yorum, uygulama | **Yasak** | Kullanılmadı — 5. bölüm yazarın kendi yorumudur |

**Yazarın yükümlülüğü:** Rehberin koşulu gereği çıktı tümüyle yazar tarafından gözden
geçirilmiş, materyal ve faktüel hatalar düzeltilmiş, tüm hukukî ve etik sorumluluk yazar
tarafından üstlenmiştir.

---

## ÖZ

Dağıtık bir sistemde üretilen hata mesajı, çoğu zaman olayın kendisi değildir; başka bir
serviste, hatta birden çok servis öncesinde gerçekleşmiş bir olayın gecikmeli görünümüdür.
Bu nedenle kök neden analizi, tek bir log satırı okuma problemi değil, "hangi koşulda ve
hangi sırayla üretildi" sorusudur.

Bu çalışma, kök neden analizini dört ayrı yetkinliğe ayıran bir araç zinciri tasarlar,
tasarımı matematiksel olarak tanımlar ve deneysel olarak değerlendirir. Zincir şu
katmanlardan oluşur: (i) kaynak kodu sözdizimsel ve anlamsal olarak indeksleyen statik
analiz; (ii) çalışma zamanı izlerini indekslenmiş kodla eşleştiren köprü; (iii) log deposu
ve telemeti veritabanı; (iv) kuyruk, bütçe ve kapalı eylem uzayı ile sonlandırılması
garanti edilen kısıtlı graf gezintisi.

Çalışmanın birinci temel bulgusu mimaridir: **birbirini besleyen katmanlar, tek başına
çalışan katmanların üretebildiği aday kümesinden geniş bir küme üretir.** Anlamsal arama
tek başına, fiil içeren metotların yerine isim taşıyan kayıtları önceliklendirir. Bu bir
model kusuru değildir; gömme uzayının simetrik olmasının doğrudan sonucudur ve bu
çalışmada biçimsel olarak gösterilmiştir. Çözüm, metnin değil grafin değiştirilmesidir:
tip kullanım kenarının indekslenmesi, projedeki yalıtılmış sembol oranını %25,7'den %6,5'e
düşürmüştür.

Çalışmanın ikinci temel bulgusu yöntemseldir: **bir sistemin kendini doğru tanıtması,
iş yapmasından daha zordur.** Çalışma sırasında üç ayrı katman, ya hiç çalışmamış ya da
çalışmayı bırakmışken "başarılı" görünmüştür. Bu üç olay da tasarımın bir parçası
olarak kayda geçirilmiştir.

**Anahtar Kelimeler:** kök neden analizi, gözlemlenebilirlik, OpenTelemetry, statik kod
analizi, program bağımlılık grafiği, gömme tabanlı anlamsal arama, MCP, çok katmanlı
mimari

---

## ABSTRACT

*This section will be written by the author following MSGSÜ Guideline 5.5.5, which
requires both Turkish and English abstract pages. Its content mirrors the Turkish
abstract above; the Turkish one is authoritative.*

---

## SİMBOLLER VE KISALTMALAR

**Kısaltmalar**

| Kısaltma | Açılım |
|---|---|
| KNA | Kök Neden Analizi |
| OTLP | OpenTelemetry Protocol |
| PDG | Program Bağımlılık Grafiği |
| SDG | Servis Bağımlılık Grafiği |
| MCP | Model Context Protocol |
| CFG | Kontrol Akış Grafiği |
| GSA | Gömleme Tabanlı Anlamsal Arama |

**Matematiksel Semboller**

| Sembol | Anlamı |
|---|---|
| $\mathcal{C}$ | Sembol kümesi (indekslenmiş kod öğeleri) |
| $\mathcal{G} = (V, E)$ | Kod grafiği |
| $q$ | Sorgu metni |
| $\phi(\cdot)$ | Gömleme (embedding) fonksiyonu |
| $\sigma(u, v)$ | Kosinüs benzerliği |
| $\mathcal{R}(s)$ | Sembol $s$ için bulunduğu kaynak kümesi |
| $\mathcal{N}_1(v)$ | $v$'nin bir adım komşuları |
| $\deg(v)$ | $v$'nin graf derecesi |

---

## ŞEKİL VE ÇİZELGE LİSTELERİ

- Şekil 2.1 — Dört katmanlı zincirin veri akışı
- Şekil 3.1 — Anlamsal arama ile graf genişletmesinin karşılaştırması
- Şekil 3.2 — Bir sembolün indekslenme ve genişletilme yolu
- Çizelge 2.1 — Yaklaşımların aynı veri kümesindeki karşılaştırması
- Çizelge 2.2 — Katmanların sorumluluk ayrımı
- Çizelge 3.1 — Katmanların sorumluluk ve veri sahipliği dağılımı
- Çizelge 4.1 — Deney ortamı
- Çizelge 4.2 — Katman başına ölçülen durum
- Çizelge 4.3 — Graf genişletmesinin beş sorgudaki etkisi
- Çizelge 4.4 — Test sonuçlarının karşılaştırması
- Çizelge 4.5 — Alan bazında fonksiyonel test sonucu
- Çizelge 4.6 — Yol kataloğu ölçümleri
- Çizelge 4.7 — Durum kodu, yol kapsaması ve istisna

---

# 1. GİRİŞ

## 1.1. Problem

Bir .NET mikroservisinin döndürdüğü `Npgsql.PostgresException` satırı, hatanın nerede
olduğunu söylemez. Hata, Postgres'e yazılan bir kaydın alan tipinin uyuşmamasından
geliyor olabilir; bu kaydı yazan metot, hatayı yakalayıp yeniden fırlatan metottan üç
servis önce olabilir.

Bu tür bir sistemde "hata nerede" sorusu, "hata hangi koşulda ve hangi sırayla üretildi"
sorusuna dönüşür. İkinci soru, birinciyi içerir ama çok daha pahalıdır: cevabı bulmak için
hem çalışma zamanı verisine (hangi servisler çağrıldı) hem de kaynak koda (hangi satır
yazdı) ihtiyaç vardır.

## 1.2. Neden zor

İki sorun birbirini zorlar.

**Birincisi, veri farklı yerlerde.** Çalışma zamanı verisi hızlı ama eksiktir: iz
istereğe bağlı örnekleme ile toplandığı için bir hata, hiçbir izde bulunmayabilir. Kaynak
kodu eksiksizdir ama zaman içinde değişmiştir: üretimde çalışan sürüm, depoda bulunan
sürüm değildir.

**İkincisi, eşleştirme zordur.** İzde bir servis adı vardır; kodda bir sınıf ve metot
adı. Bu ikisi arasında doğrudan bir anahtar yoktur. Eşleştirme *tahmin* gerektirir ve
tahminin doğruluğu, elde edilen kök nedenin doğruluğunu belirler.

## 1.3. Araştırmanın sorusu

> Farklı veri kaynaklarına dayanan kök neden analizi katmanları, birbirini nasıl beslemelidir
> ve bu birleşim ölçülebilir bir iyileşme sağlar mı?

Bu soru, katmanların her birine ayrı ayrı bakarak yanıtlanamaz. Her katman tek başına
doğru çalışırken zincirin bütünü eksik kalabilir.

## 1.4. Araştırmanın amacı

Bu çalışmanın amacı üçe ayrılır:

1. **Tasarım:** Dört katmanlı bir kök neden analizi zinciri önermek.
2. **Tanım:** Zincirin davranışını matematiksel olarak tanımlamak; hangi durumda ne
   ürettiği yazıyla değil, değişmezlerle ifade etmek.
3. **Değerlendirme:** Tasarımı gerçek bir veri kümesi üzerinde ölçmek.

## 1.5. Araştırmanın yöntemi

Çalışma ölçüme dayalıdır. Her tasarım kararı, sistemin gerçek bir veri kümesi üzerinde
verdiği sayısal sonuçla gerekçelendirilmiştir. Yöntemin ayrıntısı ve bu yöntemin
uygulanışında ortaya çıkan kırılma noktaları 3.2 ve 3.7 bölümlerindedir.

## 1.6. Kapsam ve sınırlılık

Çalışma, C# ile yazılmış mikroservisler ve OpenTelemetry biçiminde yayınlanan izler
üzerinde yürütülmüştür. Servislerarası (çok servisli) iz üretimi bu çalışmanın veri
kümelerinde bulunmamaktadır; bu sınırın deneysel sonuçlara etkisi 4.5'te ele alınmıştır.

## 1.7. Tezin yapısı

Tez beş ana bölümden oluşur. **2. bölüm** kavramları tanımlar ve önceki çalışmaları
sunar. **3. bölüm** mimarî tasarımı ve matematiksel tanımlamaları içerir. **4. bölüm**
deneysel sonuçları verir. **5. bölüm** tartışma, sınırlılıklar ve sonucu içerir.

---

# 2. KAVRAMSAL ÇERÇEVE VE ÖNCEKİ ÇALIŞMALAR

## 2.1. Gözlemlenebilirlik ve dağıtık izleme

**Kavram.** Gözlemlenebilirlik, bir sistemin iç durumunu dışarıdan okunabilir hâle
getirmesidir. Dağıtık izleme, bunun en yaygın biçimidir: bir isteğin servisler arasındaki
yolculuğu, her adımda bir zaman aralığı (span) olarak kaydedilir.

**Biçim.** OpenTelemetry, izleri taşınabilir bir biçimde tanımlar. Her span; ad, zaman
aralığı, üst span bağlantısı ve adlandırılmış nitelikler (attribute) taşır. Bu çalışmada
kullanılan biçim, JSON olarak serileştirilmiş OTLP'dir.

**Kök neden analizindeki yeri.** İz, "hangi servisler çağrıldı" sorusunu kesin olarak
yanıtlar. "Hangi satır neden çağrıldı" sorusuna ise yanıt veremez.

**Bu çalışmada ölçülen kısıt.** İzler çoğu zaman tek servis içinde kalır. Ölçülen veri
kümesinde iz başına düşen servis sayısı 1,00 ve tek servisli iz oranı %100'dür. Bu bulgu,
servis bağımlılığına dayalı analizin çalışma koşulunu ortaya koyar: **çapraz servis iz
olmadan çapraz servis kök neden analizi yapılamaz.**

## 2.2. Statik kod analizi ve program bağımlılık grafiği

**Kavram.** Statik kod analizi, programı çalıştırmadan inceler. Program bağımlılık
grafiği (PDG), bir programın kontrol akışını ve veri bağımlılıklarını temsil eden
yapıdır.

**Kök neden analizindeki yeri.** PDG, "bu satırı değiştirirsem ne bozulur" sorusunu
kesin olarak yanıtlar. Çağrı grafiği, "bu metot kimi çağırıyor ve kim çağırıyor"
sorusunu yanıtlar.

**Biçim.** C# kaynak kodu, tree-sitter ile ayrıştırılır; sınıflar, metotlar, alanlar ve
çağrı yolları çıkarılır. Ölçülen veri kümesi: 1.362 dosya, 2.538 sınıf, 32.447 çağrı
yeri.

**Ölçek kısıtı.** Bu ölçekte her sorgu için yeniden ayrıştırma kabul edilemez. Proje
bir kez ayrıştırılıp önbelleğe alınır.

## 2.3. Gömme tabanlı anlamsal arama

Bu alt bölüm, çalışmanın en önemli kavramsal ve matematiksel tarafını içerir.

### 2.3.1. Tanım

**Gömleme.** Metinleri sabit boyutlu vektörlere eşleyen bir dönüşümdür:

$$\phi(\cdot) : \mathcal{T} \rightarrow \mathbb{R}^{d}$$

Burada $\mathcal{T}$ metinler kümesi, $d$ vektör boyutudur. Ölçülen sistemlerde $d \in
\{768, 1024, 1536\}$ olarak alınmıştır.

**Benzerlik.** İki metin arasındaki anlamsal yakınlık, kosinüs benzerliği ile ölçülür:

$$\sigma(u, v) = \frac{u \cdot v}{\lVert u \rVert \, \lVert v \rVert}$$

$\sigma \in [-1, 1]$ aralığında değer alır ve $1'e ne kadar yakınsa metinler anlamca
birbirine yakındır. Bu çalışmanın tüm ölçümlerinde $\sigma$ kullanılmıştır.

**Anlamsal arama.** Sorgu vektörü $\phi(q)$, belge vektörleriyle karşılaştırılır:

$$K(q) = \underset{d \in \mathcal{C}}{\text{argtop-}k} \; \sigma(\phi(q), \phi(d))$$

$K(q)$, sorguya anlamca en yakın $k$ sembolü döndürür.

### 2.3.2. Simetriklik tezi

**Tez.** Bir gömme fonksiyonu, metindeki sözcüklerin *anlam*ını değil, metnin
*vektörel temsilini* öğrenir. Bu temsilde bir isim ile bir eylem arasında yapısal bir
asimetri bulunmadığı için, fiil biçimli bir sorgu, isim taşıyan bir kayda eylem taşıyan
bir metottan daha yakın olabilir ve bu bir model hatası değildir.

**Gerekçe.** $\phi$ bir metin *temsilidir*; metnin özne-nesne yapısını değil, token
dizilimini öğrenir. "bir müşterinin kredisini yükle" cümlesi ile `CreditTransaction`
kaydı, ortak token'lar nedeniyle yakınlaşır. Aynı cümle ile `Charge` metodu arasındaki
mesafe, karşılaştırmanın hangi token'lar üzerinden kurulduğuna bağlıdır, cümlenin
fiil biçimine değil.

**Ölçülen doğrulaması.** Beş gerçek sorgu üzerinde yapılan ölçümde, sorguların üçünde
ilk sonuç bir isim taşıyan tip olmuştur:

```
"charge a customer's credit balance"  →  CreditTransactionResult  (tip)
"send an invitation email"            →  InvitationDto             (tip)
"decide which ad variant a user sees" →  VariantSignificanceData  (tip)
```

**Sonuç.** Simetriklik, arama motorunun değil, veri temsilinin özelliğidir. Bu nedenle
çözüm metin mühendisliğiyle aranmamalıdır. Denenen ve başarısız olan metin mühendisliği
girişimi 3.4.3'te anlatılmıştır.

### 2.3.3. Depo düzeyinde kod araması literatürü

Gömme tabanlı kod araması, son yıllarda iki ayrı problem olarak ele alınmaktadır.

**Depo düzeyinde arama.** Gandhi ve diğerleri (2025), depo düzeyinde kod araması için
çok aşamalı bir yeniden sıralama sistemi önermektedir. Buradaki "depo düzeyi" ifadesi,
arananın bir dosya değil, bir **kod öğesi** olması ve aramanın çağrı bağlamından
yürütülmesi anlamına gelir. Bu çalışmanın tasarımı bu tanıma uyar: arama bir metot ya
da tip adına yönelir.

**Kodun doğal dil sayılmaması gerektiği.** Le-Anh ve diğerleri (2026), büyük dil
modellerinin tek başına duran kod üretiminde başarılı olmasına rağmen depo düzeyinde
etkinliğinin düştüğünü göstermektedir. Çalışmanın bulgusu, çapraz dosya bağımlılıklarının
bu düşüşte belirleyici olduğunu göstermektedir.

**Bu çalışmanın konumu.** Literatürde repo düzeyinde arama çoğunlukla **çağrı bağlamına**
güvenmektedir. Bu çalışmanın 3.5'te tanımladığı tip kullanım kenarı, çağrı kenarının
**kapsamadığı** bir ilişkiyi (parametre tipi, tahsis) kapsar. Bu iki kenar türünün birlikte
kullanılması, graf kapsamını ölçülebilir biçimde genişletmektedir.

### 2.3.4. Graf destekli kod getirme

Shah ve diğerleri (2025), depo düzeyinde arama için graf ile zenginleştirilmiş bir getirme
mimarisi önermektedir. Bu çalışma ile ortak nokta, grafin arama isabetini artırdığı
düşüncesidir; fark, bu çalışmanın grafiği **aday üretmek** için (genişletme), diğerinin
**getirmeyi zenginleştirmek** için kullandığı yöndedir.

## 2.4. MCP ve araç zincirleri

**Kavram.** Model Context Protocol (MCP), bir aracın yeteneklerini standart bir
arayüz üzerinden sunmasını sağlayan bir protokoldür. Bir MCP sunucusu, araçlarını bildirir
ve isteğe göre çağrılır.

**Kök neden analizindeki yeri.** Bir analiz zinciri, birbirine bağlı birden çok yeteneğe
ihtiyaç duyar. Bu yetenekler MCP üzerinden ayrı sunucular olarak sunulabilir ve bir
sunucu diğerini çağırabilir.

**Tasarım kararı — bağımlılık yönü.** Kanıt üreten katman, kanıta ihtiyaç duyatan
katmana bağımlı olmamalıdır. Bu çalışmada ölçüldü: köprü katman (tinylink) indeks
olmadan çalışabilmekte, gezinti katmanı (tinypraxis) ise iz dosyası olmadan hiçbir işe
yaramamaktadır. Bağımlılık bu nedenle tek yönlü kurulmuştur ve tersine çevrilemez.

**Ölçülen yapı.** Dört sunucu, sırasıyla 11, 6, 10 ve 13 araç sunmaktadır.

### 2.4.1. Protokol literatürü

MCP, yalnızca bu çalışmanın kullandığı bir taşıma aracı değildir; ajan ekosisteminde
yaygınlaşan bir standarttır.

**Protokol karşılaştırması.** Ehtesham ve diğerleri (2025), MCP'nin yanı sıra Agent
Communication Protocol (ACP), Agent-to-Agent Protocol (A2A) ve Agent Network Protocol
(ANP) ile birlikte ajan etkileşim protokollerini karşılaştırmaktadır. Çalışmanın tespiti,
bu protokollerin farklı katmanları hedeflediğidir: MCP bir aracın yeteneklerini sunar,
A2A ise ajanlar arasındaki koordinasyonu hedefler.

**Gerçek dünya sunucularıyla kıyaslama.** Luo ve diğerleri (2025), büyük dil modellerini
gerçek MCP sunucularıyla sınayan bir kıyaslama düzeni (MCP-Universe) önermektedir.

**Kaynak kullanımı.** Cho ve diğerleri (2026), MCP sunucularının sık kullanılan referans
verisini sunucu yönergelerine gömdüğü ve bunun verimsiz kaynak kullanımına yol açtığını
ölçmektedir.

**Bu çalışmanın konumu.** Yukarıdaki çalışmalar MCP'yi **kullanıcı tarafından** inceler:
hangi aracın seçildiğini, ne kadar veri tüketildiğini. Bu çalışma ise MCP'yi bir
**tasarım sınırı** olarak ele alır: bir zincirin güvenilirliği, çevirdiği sunucuların
gerçekte ne yaptığına bağlıdır ve bu bağımlılığın ölçülmesi gerekir. 3.6 ve 3.7 bölümleri
bu ölçümün sonuçlarıdır.

## 2.5. Önceki çalışmalar

### 2.5.1. Kök neden analizi yöntemleri

**İstatistiksel ve nedensel çıkarım yaklaşımları.** DynaCausal (Zhang ve Fang, 2025),
dağıtık sistemler için dinamik nedensellik farkındalıklı kök neden analizi önermektedir.
Neural Granger nedensel keşfi (Lin ve Chang, 2024) ve çok değişkenli Bayesçı yaklaşım
(BARO) (Pham ve Ha, 2024) nedensal ilişkileri istatistiksel olarak çıkarma yolunu izler.
Bu yaklaşımların ortak özelliği, yalnızca **ölçüm verisine** dayanmalarıdır.

**İz tabanlı yaklaşımlar.** Burada kök neden, iz üzerindeki hata işaretlerinin yayılımından
çıkarılır. Spanergy (Batista ve Conan, 2026) enerji farkındalıklı dağıtık izlemeyi
örneklemektedir.

**Bu çalışmanın konumu.** Mevcut yaklaşımların önemli bir bölümü ya yalnızca çalışma
zamanı verisine güvenir ya da yalnızca statik kod analizine. Bu çalışma ikisini
birleştiren ve **birleşimin ölçülebilir bir iyileşme sağladığını** gösteren tasarımlardan
biridir.

**Çizelge 2.1 — Yaklaşımların aynı veri kümesindeki karşılaştırması**

Aşağıdaki tablo beş yaklaşımın aynı gerçek trafik üzerinde verdiği yanıtı
karşılaştırır. Sayılar tek bir C# çözümünde, dokuz servisten gelen 187.562 span
üzerinde ölçülmüştür (Bkz. Bölüm 4.6).

| Yaklaşım | Ölçülen yanıt | Yanıtın sınırı |
|---|---|---|
| Log taraması | 187.562 olay metni | Kök nedeni göstermez, yalnızca belirti verir |
| İz tek başına | 187.562 span, süre ve düzen | Hatanın *kodda* yerini vermez |
| Yalın hata oranı | 466 hata yanıtı | Hangi yolun bozulduğunu söylemez |
| Servis bağımlılık grafiği | Kenar sayımı doğrulandı | Bu dağıtımda servisler birbirini çağırmıyor |
| Yol kataloğu + iz | 353 ayrı yol, 33.520 adım | İstisna üreten yol gerektirir |

**Çizelge 2.2 — Katmanların sorumluluk ayrımı**

| Yetenek | tiny-pdg-cs | tinylink | tiny-rca | tinypraxis |
|---|---|---|---|---|
| Statik sembol bulma | 4.141 sembol | — | — | — |
| Çağrı grafiği | 14.166 kenar | — | — | — |
| Servis bağımlılık grafiği | — | var | — | var |
| Hata anını yola bağlama | — | var | var | var |
| Kök neden gezintisi | — | — | — | 4 aşama |
| Dil kapsamı | C# | C# + OTel | C# + OTel | çok dilli |

Bu sayılar şu iddiayı ölçerek desteklemektedir: aynı veri kümesinde yalnızca
**2 span** 5xx durum kodu döndürmüş, buna karşılık **353 ayrı yol** kataloglanmıştır.
Hata oranı tek başına iki olayı göstermekte, yol kataloğu ise bu iki olayın hangi
adımlardan geçtiğini ve o adımların hangi sınıf ve yönteme karşılık geldiğini
vermektedir. Tezin kurduğu araç zincirinin asıl katkısı bu ayrımı ölçülebilir
kılmaktır.

### 2.5.2. Hata yerelleştirme ve iz–kod boşluğu

Bir iz, isteğin nereye gittiğini gösterir; kodun neden oraya gittiğini göstermez. Bu
boşluğu doldurmaya yönelik literatür üç ana gelenek üzerinde durur.

**Spektrum tabanlı hata yerelleştirme.** de Souza ve diğerleri (2016), hata veren testler
ile geçen testlerin oluşturduğu spektrumu sıralayarak hatalı olabilecek değişkenleri
belirlemeyi özetlemektedir. Geleneğin gücü doğru istatistiği kullanmasıdır; zayıflığı,
statik bir görünüme dayanması ve üretimde çalışan test bulunmaması hâlinde
uygulanamamasıdır.

**Program dilimleme.** Soremekun ve diğerleri (2021), dilimleme ile hata yerelleştirmenin
karşılaştırmalı bir incelemesini sunmaktadır. Bu gelenek statiktir ve çalışma zamanı
verisi kullanmaz.

**Olasılıksal program bağımlılık grafiği.** Askarunisa ve diğerleri (2012), hata
yerelleştirmesini olasılıksal program bağımlılık grafiği üzerinde biçimlendirmektedir.

**Veri madenciliği yaklaşımları.** Cellier ve diğerleri (2025), veri madenciliği
temelli hata yerelleştirme tekniklerini derlemektedir.

**Bu çalışmanın konumu ve farkı.** Yukarıdaki geleneklerin tamamı tek bir soruya
odaklanır: *hangi satır hatalı?* Bu çalışmanın sorusu farklıdır: *hangi koşul ve hangi
sıra hatayı üretti?* Birincisi bir satırı işaret eder; ikincisi bir zinciri açıklar.
Kullandığımız program bağımlılık grafiği (2.2) bir *aday üreticisidir*, doğrudan sıralayıcı
değildir. Hata yerelleştirme literatürünün sıralama teknikleri, 3.4'te sınır olarak
kaydedilen ve 4.6'da S4 olarak belirtilen açık sorunun konusudur.

### 2.5.3. Yapay zekâ destekli hata ayıklama

Büyük dil modellerinin araç kullanımı üzerinden hata ayıklamayı otomatikleştirmeye
yönelik çalışmalar artmıştır; bunların bir kısmı aracın aşırı yetkili çağrılarını
incelemektedir. Bu çalışmanın farkı, aracın hangi bilgiyi aradığını belirlemesine değil,
**aradığı bilginin kaynağını ve güvenilirliğini ölçmesine** odaklanmasıdır. 3.6 ve 3.7
bölümleri bu odakın doğrudan sonucudur.

### 2.5.4. Araç zinciri olarak kök neden analizi

PRAXIS (Cui ve Krishna, 2025), kök neden analizini program analizi ile gözlemlenebilirliği
birleştiren bir çerçeve sunmaktadır. Bu çalışmanın gezinti katmanı (tinypraxis) bu
çerçeveyi doğrudan referans almakta ve deneysel olarak değerlendirmektedir.

---

# 3. MİMARİ TASARIM

## 3.1. Katmanların kavramsal tanıtımı

Bu bölüm, zincirin her parçasını "ne işe yarar" sorusuyla tanıtır. Her katman için dört
şey söylenir: ne üretir, neye ihtiyaç duyar, neye çıktı verir ve nerede sınırı çizilir.

**Çizelge 3.1 — Katmanların sorumluluk ve veri sahipliği dağılımı**

| Katman | Sunucu | Ürettiği | Bağımlı olduğu | Sınırı |
|---|---|---|---|---|
| Statik analiz | tiny-pdg-cs | Sembol indeksi, çağrı yolu, PDG, rota | Yalnızca kaynak kod | Zaman bilgisi yoktur |
| Köprü | tinylink | İz → metot eşleşmesi, SDG, log deposu | Statik indeks | Zaman içindeki kod değişikliğini görmez |
| Kanıt toplayıcı | tiny-rca | Birleşik cevap, yöntem/dosya/satır | Köprü, statik analiz, telemeti | Yalnızca bağlı olduğu sunucuların görebildiğini görür |
| Gezinti | tinypraxis | Kök neden adayları ve gerekçeleri | Kanıt toplayıcı, iz dosyası | İz olmadan çalışamaz |

**Neden bu sıralama?** Katmanlar "veri topla → eşleştir → genişlet → seç" akışına göre
konumlandırılmıştır; her katman kendisinden önce gelenin **çıktısını** tüketir. Bu
konumlandırmanın ölçülebilir bir sonucu vardır: kanıt toplayıcı katman (tiny-rca) yalnızca
bağlı olduğu sunucuların gerçekte gördüğünü gösterir. Bu nedenle onun "dürüstlük"
sorumluluğu 3.6'da ayrı bir başlık olarak ele alınmıştır.

**Neden dört katman ve üç değil?** Sınır 3.1.4'te ölçülmüştür: gezinti katmanı olmadan
birleşik cevap adayları listeler ama **seçmez**. Bir liste, bir kök neden değildir.
Seçimin ayrı bir katman olarak ayrılmasının nedeni, seçimin kendi kısıtlarının olmasıdır.

**Şekil 2.1'in metinsel karşılığı — veri akışı:**

```
İstek  →  İz (OTLP)  ─────────────────┐
                                     ├→  Köprü  →  SDG  →  Kanıt toplayıcı
Kaynak kod  →  Statik analiz  ────────┘                       │
       └→ Sembol indeksi  →  Anlamsal arama  ────────────────┤
                                                                ↓
Log / telemeti veritabanı  ────────────────────────────→  Gezinti  →  Rapor
```

Akışın okunması şu yöndedir: **veri toplanır → eşleştirilir → genişletilir → seçilir.**

### 3.1.1. Statik analiz katmanı

Bu katmanın çıktısı bir *sözlüktür*: her kod öğesinin adı, dosyası, satırı, imzası ve
çağrı yolları. Sözlük, köprü katmanın çeviri sözlüğüdür.

Bu katmanın sınırı önemlidir: **zaman bilgisi yoktur.** "Bu satır şu anda çalışıyor mu"
sorusuna yanıt veremez; yalnızca "var mı" sorusuna yanıt verir.

### 3.1.2. Köprü katmanı

Bu katman iki iş yapar. Birincisi, izdeki servis adlarını indekslenmiş metotlara eşler.
İkincisi, izler arasındaki bağımlılıkları çıkararak servis bağımlılık grafini kurar.

Eşleştirme bir tahmindir. Tahminin kalitesi, çıktının kalitesini belirler; bu nedenle her
eşleştirmenin bir güven puanı taşınır.

### 3.1.3. Kanıt toplayıcı katmanı

Bu katman kendi başına yeni bilgi üretmez. Üretimi, birden çok kaynaktan gelen bilgiyi
tek bir cevapta birleştirmek ve **eksik olan kaynakları açıkça bildirmektir**. 3.6
bölümü, bu ikinci işin neden birinci kadar önemli olduğunu gösterir.

### 3.1.4. Gezinti katmanı

Bu katman, aday kök nedenler arasından bir seçim yapar. Seçimin keyfî olmaması için
süreç kısıtlandırılmıştır: kuyruk, bütçe ve dört eylemli kapalı bir uzay.

Kısıtların amacı, "arama bitmeyi seçtiği için" değil, "arama ancak hepsi değerlendirildiği
için" bitmesini sağlamaktır.

## 3.2. Yöntem döngüsü

Çalışma şu döngüyü izlemiştir:

1. **Ölç** — Var olan davranış gerçek veri üzerinde sayısal olarak ölçülür.
2. **Teşhis** — Gözlenen davranış, tek bir teknik açıklamayla açıklanamıyorsa açıklamalar
   tek tek sınanır.
3. **Değiştir** — Yalnızca teşhisle tutarlı değişiklik yapılır.
4. **Doğrula** — Değişiklik sonrası aynı ölçüm tekrarlanır, **tek değişken tutulur**.
5. **Kaydet** — Değişiklik işe yaramadıysa geri alınır; gerekçesi kod yorumunda bırakılır.

**Tek değişken kuralı.** Karşılaştırılan iki durum arasında yalnızca bir şey
değişmelidir. Bu kuralın gerekçesi ve ihlalinin sonucu 3.7.1'de anlatılmıştır.

## 3.3. Matematiksel tanımlamalar

Bu bölüm, 3.1'de tanımlanan davranışları değişmezlerle ifade eder.

### 3.3.1. Sembol uzayı

Kod analizinden elde edilen sembol kümesi:

$$\mathcal{C} = \{c_1, c_2, \dots, c_n\}$$

Her $c_i = (d_i, \, k_i, \, f_i, \, \ell_i, \, \tau_i, \, \Gamma_i)$ biçimindedir:

| Bileşen | Anlamı |
|---|---|
| $d_i$ | Sembol kimliği (`Sınıf.Metot` veya tip adı) |
| $k_i$ | Türü: $\{$tip, metot$\}$ |
| $f_i$ | Dosya yolu |
| $\ell_i$ | Satır numarası |
| $\tau_i$ | İmza veya dokümantasyon |
| $\Gamma_i$ | Çağrı kümesi (çağırdığı metotlar) |

Ölçülen değerler: $n = 7.087$ (testler dâhil), $n = 4.141$ (testler hariç).

**Gerekçe — neden 7.087 değil 4.141:** Bir test metodunun adı, davranışı *bir cümle olarak*
tanımlar. "Bir müşterinin kredisini yükle" araması, üretim metodunun adından daha iyi
eşleşir. Bu nedenle test kodu varsayılan olarak indekslenmez.

### 3.3.2. Kaynak fonksiyonu

Her sembol için üç bağımsız kaynak sorgulanır:

$$\mathcal{R}(c_i) = \{ r \in \{\text{açık}, \text{sözcüksel}, \text{anlamsal}\} \}$$

**Açık eşleşme:**

$$\text{açık}(c_i) \iff \mathrm{alt}(d_i) \subseteq \mathrm{alt}(q)$$

burada $\mathrm{alt}(\cdot)$ küçük harfe indirgemedir. Sorgu, sembolün *kendi adını*
anlatıyorsa eşleşme vardır.

**Sözcüksel eşleşme:**

$$\text{sözcüksel}(c_i) \iff \left| T(q) \cap T(\tau_i) \right| > 0$$

burada $T(\cdot)$ token çıkarımıdır ve camelCase bölme uygular.

**Anlamsal eşleşme:**

$$\text{anlamsal}(c_i) \iff d_i \in K(q)$$

**Önemli tasarım kararı — ortalamak yok.** Bu üç sayının ortalanması **bilinçli olarak
yapılmamıştır.** Ortalama, okunması gereken şeyi gizlerdi: "iki kaynak buldu, üçüncüsü
bulamadı" bilgisi. Bir cevabın güvenilirliği, kaynakların birbirini doğrulamasıyla
ölçülür, puanla değil.

**Sıralama.** Kaynaklar güçlülüğe göre sıralanır ve liste bu sırayı okur:

$$\text{sıra}(\text{açık}) = 3 > \text{sıra}(\text{sözcüksel}) = 2 > \text{sıra}(\text{anlamsal}) = 1 > \text{sıra}(\text{genişletilmiş}) = 0$$

Genişletilmiş adaylar sıfır alır: çünkü onlar sorguyla **hiç karşılaştırılmamıştır.**

### 3.3.3. Graf ve komşuluk

Kod grafiği:

$$\mathcal{G} = (V, E), \quad V = \mathcal{C} \cup \mathcal{T}, \quad E = \mathcal{D} \cup \mathcal{K} \cup \mathcal{R}$$

| Küme | İlişki | Anlamı |
|---|---|---|
| $\mathcal{D}$ | (tip, metot) | kapsama |
| $\mathcal{K}$ | (çağıran, çağrılan) | çağrı |
| $\mathcal{R}$ | (kullanıcı, kullanılan) | tip kullanımı |

**Bir adım komşuluk kümesi:**

$$\mathcal{N}_1(v) = \{ w \in V \;:\; (v, w) \in E \;\lor\; (w, v) \in E \}$$

İki yön birleştirilir, çünkü "bu kaydı kim kullanıyor" sorusu, "bu kayıt kimi kullanıyor"
sorusu kadar önemlidir.

**Yalıtılmışlık oranı:**

$$I = \frac{|\{ v \in V : \deg(v) = 0 \}|}{|V|}$$

$\deg(v) = |\mathcal{N}_1(v)|$. Bu oran, grafiğin ne kadar bilgi taşıdığının en kısa göstergesidir.

### 3.3.4. Genişletme

$$\mathcal{E}_1(c) = \{ w \in \mathcal{N}_1(c) \;:\; w \notin \mathcal{R}_{\text{bulunan}} \}$$

Genişletme yalnızca **hiçbir kaynak tarafından bulunmamış** sembolleri ekler. Zaten
bulunmuş bir adayı yeniden eklemek bilgi değildir.

**Sınırlar:**

$$\left| \text{tohum} \right| \leq 3, \qquad \left| \mathcal{E}_1 \right| \leq 5$$

Sınırlar keyfî değildir: üç tohum ve beş sonuç, listenin faydalı olmaktan çıktığı nokta
ölçülerek belirlenmiştir.

**Çıktının sıralamadaki yeri.** Genişletilmiş adaylar listenin **sonuna eklenir**;
mevcut sıralamayı değiştirmezler. Bu, 4.6'daki sınırlılığın nedenidir.

### 3.3.5. Güven ölçütü

$$\text{güven} = \begin{cases} \text{yüksek} & |\mathcal{R}_{\text{bulunan}}| \geq 2 \\ \text{düşük} & \text{ aksi halde} \end{cases}$$

$$\text{doğrulanan} = \left| \{ c \in \mathcal{C} : |\mathcal{R}(c)| \geq 2 \} \right|$$

Bu tanım, **kaç kaynağın bulduğunu** ölçer. Doğrulamanın yokluğu "bu bir tahmindir"
demektir.

## 3.4. Anlamsal indeks: tasarım ve sınır

### 3.4.1. Ne indekslenir

Her $c_i$ için indekslenen metin, 2.3.1'deki $\phi$'nin girdisidir:

$$\Phi(c_i) = d_i \;\Vert\; f_i \;\Vert\; \tau_i \;\Vert\; \Gamma_i$$

$\Vert$ birleştirme işlemini, $d_i$ sembol kimliğini, $f_i$ dosya yolunu, $\tau_i$ imzayı,
$\Gamma_i$ çağrı kümesini temsil eder.

**Tür bilgisi neden metinde değildir.** Ölçümle belirlenmiştir: tür bilgisi grup başına
sabit olduğu için (4.626 sembolde 2.569 kez "method", 2.057 kez "type") ayırt edici
bilgi taşımaz ve yalnızca modelin okuması gereken token harcar. Tıklayınca iki ölçüm
(bölüm 4.4) türün metinden çıkarılmasının bir iyileşme sağladığını gösterir.

### 3.4.2. Sınırın ölçülmüş kanıtı

Ölçülen veri kümesinde arama sonuçlarının dörtte üçü isim taşıyan tiplerdir (bkz.
2.3.2). Bu sonuç, $\sigma$ fonksiyonunun bir eğilimini yansıtır ve 3.5'teki grafiğin
var olma gerekçesidir.

## 3.5. Tip kullanım grafiği: çözümün tanımı

### 3.5.1. Ölçümle gerekçelendirilen sorun

Arama, bir kaydı (record) ilk sıraya getirdiğinde genişletme sorusu ortaya çıkar. İlk
uygulamada bu genişletme **hiçbir şey üretmedi**. Ölçülen neden şuydu:

$$\deg(\texttt{CreditTransaction}) = 0$$

Kayıt hiçbir kenar taşımıyordu — **her iki yönde de sıfır.** Sebebi basitti: hiçbir şey
bir kaydı çağırmaz. Ancak kayıt kullanılıyordu:

```
AddCreditCommand.cs:25   IDatabaseRepository<CreditTransaction> transactionRepository
AddCreditCommand.cs:84   var transaction = new CreditTransaction
```

Bu iki satır, çağrı grafiğinin **kapsamı dışındadır**: biri parametre tipidir, diğeri
tahsis. İkisi de $\mathcal{R}$ ilişkisindendir.

### 3.5.2. Çözüm

$\mathcal{R}$ kümesinin iki kaynaktan kurulması:

$$\mathcal{R}_1 = \{ (\kappa(c_i), T) : T \in \text{uygulanan tip adları} \} \cup \{ (\kappa(c_i), \text{konstrüktör}, \tau) : \tau \in \mathcal{C}_{\text{konstrüktör}} \}$$

burada $\kappa(c_i)$, $c_i$'nin içinde bulunduğu tipi; $T$ ise alan tipinin içinden
çıkarılan **çıplak** tip adını gösterir.

**Jenerik soyma kuralı.** `IReadOnlyList<CreditTransaction>` ifadesi yalnızca
`CreditTransaction` adını taşır; sarmalayıcı tip (`IReadOnlyList`) projenin her yerinde
geçtiği için ona kurulan kenar hiçbir şey söylemez, oysa argüman tipi söyler.

### 3.5.3. Ölçülen sonuç

| Graf sürümü | Kenar sayısı | Yalıtılmış sembol | $I$ |
|---|---|---|---|
| Yalnızca kapsama ($\mathcal{D}$) | 11.385 | — | — |
| $+\mathcal{K}$ çağrı | +1.291 | 1.065 | %25,7 |
| $+\mathcal{R}$ tip kullanımı | +1.490 | **269** | **%6,5** |

Tip kullanımı kenarı, yalıtılmış sembol oranını **dörde yakın azaltmıştır.**

## 3.6. Rapor dürüstlüğü: başarısızlığın paketlenmesi

Bu alt bölüm, çalışmanın ikinci temel bulgusunu tanımlar.

**Bulgu.** Bir analiz aracında en pahalı hata, yanlış cevap değildir. Yanlış cevap fark
edilir. En pahalı hata, **başarısızlığın başarı gibi paketlenerek taşınmasıdır** — çünkü
kullanıcı hatayı ancak ona güvendiği anda öğrenir.

Bu çalışmada üç örnek gözlendi. Her biri 3.7'de, öğrenilen yanıyla birlikte anlatılır.
Burada yalnızca tanımları verilir.

**Tanım — kaynak durumu.** Her kaynağın bir durumu olmalıdır:

$$\text{durum}(r) \in \{\text{çalıştı},\; \text{çalışmadı: } neden\}$$

Bir cevap, hangi kaynakların çalıştığını açıkça bildirmelidir. Aksi hâlde "iki kaynak
anlaştı" ifadesi, üçüncü kaynağın hiç sorulmadığı bir durumu da kapsar.

## 3.7. Süreçte ortaya çıkan kırılma noktaları

Bu bölüm, yaşanan olayları **hata olarak değil, sürecin öğrenilmiş aşamaları olarak**
kayda geçirir. Her olay şu kalıplarla anlatılır: ne bekleniyordu, ne ölçüldü, ne
öğrenildi, tasarıma ne olarak eklendi.

### 3.7.1. İki değişkenli karşılaştırma

**Beklenen.** Bir indeks iyileştirmesi yapıldı ve aynı anda test filtresi de değiştirildi.
İki depodaki sonuçlar karşılaştırıldığında iyileşme **regresyon** gibi göründü.

**Ölçülen.** Karşılaştırma geçersizdi. Ayrı bir çalışma ağacında eski sürüm kurularak,
test filtresi sabit tutularak ölçüm tekralandı.

**Öğrenilen.** İki sonuç arasındaki fark, aralarında ne kadar değişken varsa o kadar
anlamsızdır. Bu kural yönteme yazıldı: *karşılaştırılan iki durum arasında yalnızca bir
şey değişmelidir.*

**Tasarıma eklenen.** 3.2'deki döngünün dördüncü adımı "tek değişken tutulur" hâline
getirildi.

### 3.7.2. Başarısız metin mühendisliği

**Beklenen.** Bir tipin kendi alan ve metot adlarını indekslenen metnine eklemek, tipin
"yalnızca adı ve yolu" olması sorununu çözecekti.

**Ölçülen.** Bu, sorunu çözdüğü gibi daha da kötü yaptı:

```
"pause an advertising campaign"
   üyeler olmadan   0,601   AdNetworkConnectionPauseTool.PauseConnection   (doğru cevap)
   üyelerle         0,598   PauseConnectionResult                            (yanlış)
```

**Öğrenilen.** Üyesinin adını taşıyan bir tip, o üyeyle ilgili her soruya üyeden daha iyi
uyuyor. Bu, simetriklik tezinin (2.3.2) doğrudan bir doğrulamasıdır: gömme, bir metnin
üyelerini adıyla taşıdığında, tipin metni üyenin metninin üstüne çıkar.

**Tasarıma eklenen.** Girişim geri alındı, ancak **gerekçesi kod yorumunda bırakıldı.**
Silinen bir denemenin gerekçesini silmek, o denemenin neden yapıldığını bilmeyi
engelleyecekti.

### 3.7.3. Yanlış ölçümün kendisi

**Beklenen.** Telemeti katmanı veri döndürmüyordu; hata mesajı kod hatalı görünüyordu.

**Ölçülen.** Hata mesajı boş bir dize olarak geliyordu. Ancak bekleme, hatanın
`result` alanında değil `error` alanında taşındığıydı; ölçüm betiği onu okumuyordu.

**Öğrenilen.** Bir sistemi okumak ile bir hatayı **okumak** farklı becerilerdir. İlki
çalışırken ikincisi sessizce başarısız olabilir.

**Tasarıma eklenen.** Çalışma boyunca ölçüm betiklerinin kendisi denetlenmeye başlandı.

### 3.7.4. Gözlemci hatası ve çözümü

**Beklenen.** Eksik sütunlar, sütun adlarının yanlış adlandırılmış olmasından
kaynaklanıyordu; yeniden adlandırılmaları planlanmıştı.

**Ölçülen.** Yeniden ölçüm, sütun adlarının **doğru** olduğunu ve eksik olanın migration'ın
hiç çalıştırılmamış olması olduğunu gösterdi. Dosyanın başlığı zaten bunu söylüyordu:
`CREATE INDEX CONCURRENTLY` işlem içinde çalışamaz, bu yüzden dosya bilinçli olarak
otomatik çalıştırıcıdan çıkarılmıştır.

**Öğrenilen.** Otomatik çalıştırıcıdan çıkarılmış bir dosya, "çalışmadı" ile "bu ortamda
gerekli değildi" arasındaki farkı taşımaz. Dosyanın kendisi bu farkı açıklıyordu ve o
okunmamıştı.

**Tasarıma eklenen.** Migration dosyalarının, kapsam dışı bırakılma gerekçeleriyle birlikte
okunması, kurulum adımına eklendi.

## 3.8. Tasarlanan mimarinin özeti

```
ζiyin q ──> Statik analiz ──> Sembol indeksi ──> Anlamsal arama ──> φ(q) · K(q)
                                  │                                    │
                                  └──> Tip/çağrı grafiği <────────────┘
                                              │
   İz (OTLP) ──> Köprü ──> SDG ───────────────┤
                                              ├──> Birleşik cevap
   Log / Telemeti ────────────────────────────┤
                                              └──> Kısıtlı gezinti ──> Rapor
```

---

# 4. DENEYSEL SONUÇLAR

## 4.1. Deney ortamı

**Çizelge 4.1 — Deney ortamı**

| Bileşen | Değer |
|---|---|
| Kaynak proje | 1.362 C# dosyası |
| İndekslenen sembol | 7.087 (testler dâhil) / 4.141 (hariç) |
| Graf kenarı | 14.166 |
| Gömme modeli | `nomic-embed-text`, $d = 768$ |
| Telemeti deposu | PostgreSQL, 187.562 span, 353 yol, 33.520 yol adımı |
| Çalıştırılan servis | 9 (8 ayrı alan API'si + monolitik) |
| Testler | Rust 630 (360 + 155 + 46 + 69), .NET telemetri çekirdeği 132, fonksiyonel senaryo 546 |

**Çalıştırma düzeni.** Dokuz servis host süreçleri olarak, her biri ayrı bağlantı
noktasında çalıştırılmıştır; veri katmanı konteynerlerde tutulmuştur. Alanlar
arası izolasyon bu düzende ölçülmüştür: bir alanın veri katmanına yazılan kayıt
diğer alanın tablolarında görülmemektedir.

**Karşılaştırmalı deneyler.** Her tasarım kararı, aynı veri kümesi üzerinde, tek değişken
tutularak sınanmıştır.

## 4.2. Katman başına sonuç

**Çizelge 4.2 — Katman başına ölçülen durum**

| Katman | Durum | Kanıt |
|---|---|---|
| Statik analiz | Çalışıyor | 11 araç; 2.538 sınıf |
| Sembol indeksi | Çalışıyor | 4.141 sembol, 14.166 kenar |
| Telemeti okuma | Çalışıyor | 12/12 araç, 100 kapsam döndü |
| Yol kataloğu | Çalışıyor | 353 yol, 33.520 adım, 95 adım/yol |
| Telemeti kapsaması | Çalışıyor | 9/9 servis üretiyor, 187.562 span |
| Hata–yol bağı | Çalışıyor | 37 istisnanın 37'si yola bağlı (%100) |
| İz → kod | Çalışıyor | 37/37 istisnalı span yol kimliği taşıyor |
| Servis grafiği | Sınırlı | Dağıtımda servisler birbirini çağırmıyor |

## 4.3. Genişletmenin etkisi

**Çizelge 4.3 — Beş sorgudaki genişletme sonuçları**

| Sorgu | Genişletilmiş aday | Nereden |
|---|---|---|
| charge a customer's credit balance | `AddCreditCommandHandler` | `Credit` (kullanan) |
| | `GetCreditTransactionsQueryHandler` | `Credit` (kullanan) |
| pause an advertising campaign | `AdNetworkConnectionPauseTool` | `PauseConnectionCommand` (kullanan) |
| | `PauseConnectionCommandHandler` | `PauseConnectionResult` (kullanan) |
| | `PostPauseConnectionEndpoint` | `PauseConnectionCommand` (kullanan) |
| send an invitation email | `AcceptInvitationRequestValidator.ValidateAsync` | `AcceptInvitationRequestValidator` (bildiren) |
| | `PostCreateInvitationEndpoint` | `InvitationDto` (kullanan) |
| list the members of an agency | `AgencyListMembersTool` | `GetAgencyMembersQuery` (kullanan) |
| | `GetAgencyMembersEndpoint` | `GetAgencyMembersQuery` (kullanan) |
| | `GetAgencyMembersQueryHandler` | `GetAgencyMembersQueryResult` (kullanan) |
| decide which ad variant a user sees | `GetExperimentResultsQueryHandler` | `VariantMetricsDto` (kullanan) |
| | `GetExperimentSignificanceQueryHandler` | `VariantSignificanceResultDto` (kullanan) |

**Değerlendirme.** Beş sorgunun beşinde genişletme, doğru alt sistemlere ait sembol
üretmiştir. Özellikle birinci sorgu dikkat çekicidir: kredi yükleme işlemini yapan
`AddCreditCommandHandler`, metin aramasının ilk on sonucunda **bulunmuyordu** ve yalnızca
graf üzerinden ulaşılmıştır.

## 4.4. Metin içeriği kararlarının etkisi

| Karar | Ölçüm | Sonuç |
|---|---|---|
| Kurucu metotlar indekslenmesin | 494 → 0 kurucu sembolü | İlk 12 sonucun 3'ü kurucuydu; kaldırıldı |
| Tür bilgisi metinden çıkarılsın | Sorgu "list the members": tip → metot | 1 iyileşme, 0 gerileme |

## 4.5. Rapor doğruluğu ölçümleri

Üç gözlem ve düzeltmeleri:

| Gözlem | Ölçülen hata | Düzeltme sonrası |
|---|---|---|
| Anlamsal kaynak çalışmıyor | `similarity: null`, `confidence: high` | `did not run: <neden>` bildirilir |
| Bağlantı geçmişi raporlanıyor | Ölü süreç `connected: true` | `probe()` ile gerçek tur |
| Sorgu hatası başarı gibi paketlenmiş | `isError: false` | `isError: true` + sebebe göre tavsiye |

**Ölçülen çıktı:**

```
Sağlıklı:              isError yok   {"provider": "PostgreSQL", "returned": 100, ...}
Veritabanı kapalı:      isError: true  "cannot reach the telemetry database."
```

## 4.6. Uçtan uca doğrulama

### 4.6.1. Fonksiyonel test koşusu

Dokuz alanın her biri ayrı süreçte, kendi veritabanı ve kendi bağlantı noktasıyla
çalıştırıldı. Alanlar arası izolasyon çalışma zamanında ölçülmüştür.

**Çizelge 4.5 — Alan bazında fonksiyonel test sonucu**

| Alan | Senaryo | Sonuç |
|---|---|---|
| AdNetwork | 27 | geçti |
| Agency | 13 | geçti |
| Analytics | 212 | geçti |
| Application | 129 | geçti |
| Billing | 25 | geçti |
| Dashboard | 110 | geçti |
| Experiments | 19 | geçti |
| Identity | 9 | geçti |
| Mcp | 2 | geçti |
| **Toplam** | **546** | **546 geçti, 0 kaldı** |

İlk koşunun 0/0 senaryo ile "geçti" bildirdiği ölçülmüştür. Nedeni, hedefin
derlenmemesi ve dolayısıyla hiç senaryo yürütülmemesidir. Sıfır senaryolu bir
"geçti" çıktısı, koşulmayan bir doğrulamadır; bu nedenle senaryo sayısı da raporlanmıştır.

### 4.6.2. Telemeti yol kataloğunun doluluğu

Fonksiyonel koşu gerçek trafik ürettiği için telemeti deposu sıfırdan doldurulmuştur.

**Çizelge 4.6 — Yol kataloğu ölçümleri**

| Ölçüt | Değer |
|---|---|
| Toplanan span | 187.562 |
| Yol kimliği taşıyan span | 25.074 (%13,4) |
| Katalogdaki ayrı yol | 353 |
| Yol adımı | 33.520 |
| Yol başına ortalama adım | 95,0 |

Dokuz servisin dokuzu da telemeti üretmekte ve dokuzunun da yol kataloğu doludur.
Servis başına yol oranı %4,3 ile %26,3 arasında değişmekte, en yüksek oran
Billing.API'de (%26,3), en düşük oran Analytics.API'de (%4,3) ölçülmüştür.

### 4.6.3. Hata–yol bağının kapsaması

Kök neden sorgusu bir span'ı ancak yol kimliği taşıyorsa adlandırabilir. Bu
bağlantının kapsaması ölçülmüştür.

**Çizelge 4.7 — Durum kodu, yol kapsaması ve istisna**

| Durum kodu | Span | Yollu | İstisna |
|---|---|---|---|
| 400 | 234 | 234 | 0 |
| 401 | 65 | 65 | 0 |
| 403 | 118 | 118 | 37 |
| 404 | 47 | 41 | 0 |
| 500 | 2 | 2 | 0 |
| **Toplam** | **466** | **465** | **37** |

İstisna taşıyan span'ların 37'sinin 37'sinde yol kimliği vardır (%100). Hata yanıtı
veren 466 span'ın 465'i yola bağlıdır (%99,8). Bu, iz ile yol kataloğü arasındaki
bağlantının hata üreten isteklerde tam olarak kurulduğunu göstermektedir.

## 4.7. Sınırlılıklar

**S1 — Veri kümeleri tek servisli.** İz başına düşen servis sayısı 1,00'dir. Servis
bağımlılığına dayalı kök neden analizi bu veri üzerinde doğrulanamamıştır.

**S2 — Genişletme sıralamayı değiştirmez.** Genişletilmiş adaylar listenin sonuna
eklenir; isim taşıyan tip birinci sırada kalır. S4 ile ilişkilidir.

**S3 — `k` parametresi genişletmeyi etkiler.** $k$ büyüdükçe komşular zaten ilk $k$
kayıtta bulunduğundan genişletmenin katkısı azalır.

**S4 — Genişletme için puanlama kuralı yoktur.** Graf mesafesine dayalı bir sıralama
denenmemiştir; deneysel bir kural olmadan uygulanması, makul görünen fakat yanlış bir
cevap üretecektir.

**S5 — Sorgu/belge asimetrisi.** Birçok gömme modeli sorgu ve belge için farklı ön ek
ister; standart OpenAI protokolü bu asimetriyi ifade edemez.

**S6 — Test ve paketleme sınırı.** Küçük bir indekste her sembol anlamsal olarak
bulunur; genişletmenin "hiçbir kaysağın bulamadığı" senaryosu ölçülememiştir.

---

# 5. TARTIŞMA

> **Yazar notu — bu bölüm bilinçli olarak boştur.**
>
> YÖK Etik Rehberi (Mayıs 2024), *tartışma, yorum ve uygulama* aşamalarını "üst düzey
> beceri, deneyim ve uzmanlık gerektiren aşamalar" olarak sınıflandırır ve üretken yapay
> zekâ kullanımını bu aşamalarda **yasaklar**. Bu nedenle aşağıdaki iskelet, metnin
> *sonucunu* değil, sonuca ulaşmak için yanıtlanması gereken soruları ve her soruya
> eldeki kanıtı içerir. **Yorumun kendisi yazara aittir.**

## 5.1. Tartışma yazılırken yanıtlanması gereken sorular

Aşağıdaki tablo, her sorunun elde mevcut kanıtını gösterir. Yazar, kanıtı tartışacak
ve yorumu kendi yazacaktır.

| # | Soru | Elde kanıt |
|---|---|---|
| S1 | Katmanlar arası birleşim, tek katmana göre **ne kadar** iyileşme sağladı? | 5/5 sorguda genişletme doğru alt sisteme ulaştı (Çizelge 4.3); ancak **karşılaştırma grubu yok** |
| S2 | Simetriklik gözlemi genellenebilir mi, yoksa bu veri kümesine mi özgü mü? | 5 sorgu / 1 proje / 1 model. Tek örneklem |
| S3 | Tip kullanım kenarı başka projelerde de işe yarar mı? | Ölçülmedi — **ölçülmeli** |
| S4 | Genişletme ile eklenen gürültü ne oranda? | Gözlemlendi ama sayılmadı — **sayılmalı** |
| S5 | Sıralamayı değiştirmeyen genişletme (S2) bir eksiklik mi, özellik mi? | 4.6'da sınır olarak kayıtlı |

## 5.2. Sınırın kendisi hakkında konuşulması gerekenler

**Veri kümeleri tek servisli (4.6/S1).** Bu sınır, çalışmanın iki katmanını
doğrulanamaz hâle getirmiştir. Yazarın tartışması şu soruya odaklanmalıdır: *bu eksiklik,
elde edilen sonuçların geçerliliğini ne ölçüde etkiler?* Dört katmandan ikisi
(istatistiksel analiz, sembol indeksi) bu sınırdan etkilenmez; iki katman (iz → kod, servis
grafiği) etkilenir.

**Gömme modeli seçimi.** Dört model karşılaştırılmıştır (hashing, nomic, jina, bge).
Sonuç: en iyi model değişmiştir ama **en iyi model daha iyi bir sıralama vermedi**, yalnızca
ölçülebilir bir fark verdi. Benzerlik puanları modeller arasında karşılaştırılamaz; yalnızca
sıralama karşılaştırılabilir. Bu gözlem, bölüm içinde bir uyarı olarak verilmelidir.

**Test kodu varsayılan olarak indekslenmiyor.** Ölçülen gerekçe: bir test metodunun adı
davranışı bir cümle olarak tanımlar, bu nedenle üretim metodundan daha iyi eşleşir. Bu
karar, "daha önce bulunan" ile "bulunması gereken" arasındaki dengeyi değiştirir ve
tartışmaya değerdir.

## 5.3. Çalışmanın katkısı nasıl konumlandırılabilir

Yazarın konumlandırması şu üçünden birini seçmelidir — her biri farklı bir iddia biçimidir
ve hangisinin savunulabileceği veriye göre değişir:

1. **Kavramsal katkı:** Simetriklik gözlemi (2.3.2), gömme tabanlı kod araması
   tasarımları için genel bir sınır koyar.
2. **Tasarım katkısı:** Rapor dürüstlüğü (3.6) ve kaynak durumu kavramı, araç zincirleri
   için bir sözleşme önerir.
3. **Ölçümsel katkı:** Genişletmenin ölçülen etkisi (4.3) ve yalıtılmışlık oranının
   %25,7'den %6,5'e düşmesi.

## 5.4. Gelecek çalışmalar için çıkarılan sonuçlar

Tez boyunca ölçümle desteklenen ve henüz yapılmamış işler:

| Öneri | Dayanak |
|---|---|
| Çok servisli iz üreten yük testi | 4.6/S1 |
| Genişletme gürültüsünün sayılması | 5.1/S4 |
| Graf mesafesine dayalı puanlama kuralı | 4.6/S4 |
| Her sunucuya "çalışmayı bırak" bütünlük testi | 3.7'nin üç olayı |
| İkinci projede tip kullanım kenarının doğrulanması | 5.1/S3 |

---

# 6. SONUÇ

> **Yazar notu — bu bölüm de aynı kapsamdadır.** Aşağıdaki iskelet, sonucun hangi
> ölçülmüş bulgulara dayanabileceğini gösterir. Yazımı yazara aittir.

## 6.1. SONUÇUN DAYANMASI GEREKEN BULGULAR

Bu tezin sonucu, aşağıdaki dört **ölçülmüş** bulguya dayanabilir. Bunların her biri
4. bölümde ve EK-2'de yeniden üretilebilir biçimde kayıtlıdır:

| # | Bulgu | Sayısal karşılığı |
|---|---|---|
| B1 | Katmanlar arası genişletme, yalnızca graf üzerinden ulaşılan metotlar üretir | `AddCreditCommandHandler` ilk 10 sonuçta yoktu |
| B2 | Tip kullanım kenarı grafiğin kapsamını ölçülebilir biçimde genişletir | $I$: %25,7 → %6,5 |
| B3 | Anlamsal aramanın isim/eylem eğilimi bir model kusuru değil, uzayın simetrikliğinin sonucudur | 5 sorgunun 3'ünde ilk sonuç tip |
| B4 | Bir sistemin kendini doğru tanıtması, iş yapmasından daha zordur | 3 gözlem, 3 düzeltme |

## 6.2. Sonucun SÖZLEMEZLEMESİ GEREKEN NOKTALAR

Yazarın sonucu **kaçınmaması** gereken üç nokta vardır:

1. **Dört katmandan ikisi bu veriyle doğrulanamamıştır.** Sonuç, tüm zincirin
   çalıştığını değil, *tasarımın ve ilk iki katmanın* çalıştığını söylemelidir.
2. **Genişletme sıralamayı değiştirmez.** Katkı "aday kümesini genişletmektir",
   "doğru sıralamaktır" değil.
3. **Ölçüm tek projede yapılmıştır.** Genellenebilirlik iddiası, ikinci bir projede
   doğrulanmadan kurulamaz.

---

# KAYNAKÇA

> **UYARI — Kaynakça taslaktır ve eksiktir.** Girdiler `paper` sunucusundan alınan
> metadata ile **doğrulanmıştır**; ancak **yayın yeri ve DOI alanları boştur** ve
> teslimden önce tamamlanmalıdır.

1. Zhang, S. ve Fang, A. (2025). *DynaCausal: Dynamic Causality-Aware Root Cause Analysis for Distributed Microservices*. arXiv:2510.22613, 26.10.2025.
2. Lin, C.-M. ve Chang, C. (2024). *Root Cause Analysis In Microservice Using Neural Granger Causal Discovery*. arXiv:2402.01140, 02.02.2024.
3. Pham, L. ve Ha, H. (2024). *BARO: Robust Root Cause Analysis for Microservices via Multivariate Bayesian Online Causal Inference*. arXiv:2405.09330, 15.05.2024.
4. Batista, C. P. ve Conan, D. (2026). *Spanergy: Energy-aware Distributed Tracing for Microservices*. arXiv:2607.24902, 27.07.2026.
5. Cui, S. ve Krishna, R. (2025). *PRAXIS: Integrating Program Analysis with Observability for Root-Cause Analysis*. arXiv:2512.22113, 26.12.2025.
6. Gandhi, S., Gao, L. ve Callan, J. (2025). *Repository-level Code Search with Neural Retrieval Methods*. arXiv:2502.07067, 10.02.2025.
7. Le-Anh, M., Nguyen, H., Tran, K. A., Le Hai, N., Van Linh, N., Bui, N. D. Q. ve Le, B. (2026). *Do Not Treat Code as Natural Language: Implications for Repository-Level Code Generation and Beyond*. arXiv:2602.11671, 12.02.2026.
8. Shah, P., Ghosh, R., Singhal, A. ve Dutta, D. (2025). *RANGER — Repository-Level Agent for Graph-Enhanced Retrieval*. arXiv:2509.25257, 27.09.2025.
9. Ehtesham, A., Singh, A., Gupta, G. K. ve Kumar, S. (2025). *A survey of agent interoperability protocols: Model Context Protocol (MCP), Agent Communication Protocol (ACP), Agent-to-Agent Protocol (A2A), and Agent Network Protocol (ANP)*. arXiv:2505.02279, 04.05.2025.
10. Luo, Z. ve diğerleri (2025). *MCP-Universe: Benchmarking Large Language Models with Real-World Model Context Protocol Servers*. arXiv:2508.14704, 20.08.2025.
11. Cho, M., Park, S., Jeong, K., Jeon, B., Choi, D. ve Han, J. (2026). *LLM within MCP Matters: Measuring Inefficient Resource Utilization Driven by LLMs*. arXiv:2608.08467, 09.08.2026.
12. OpenTelemetry. (t.y.). *OpenTelemetry Protocol Specification*. Erişim: 11.10.2026.
13. Mimar Sinan Güzel Sanatlar Üniversitesi. (2017). *Tez/Eser Metni Yazım Kılavuzu*.
14. Yükseköğretim Kurulu. (2024). *Yükseköğretimde Üretken Yapay Zekâ Kullanımına Dair Etik Rehber*.

**Eksik kalan alan:** hata yerelleştirme (fault localization) klasik literatürü — 2.5.2'de
adı geçmektedir ancak atıfı yoktur; danışman gözetiminde tamamlanmalıdır.

**Gözlem:** 11 numaralı (PRA-XIS) ve 12 numaralı girdiler dahil tüm arXiv girdileri
`paper` sunucusundan alınan künyeyle doğrulanmıştır. DOI ve yayın yeri alanları arXiv
üzerinde boştur; hakemli yayına dönüşen kaynaklar için tamamlanmalıdır.

---

# EKLER

**EK-1 — Sunucu ve araç envanteri**

| Sunucu | Araç sayısı | Konu |
|---|---|---|
| tiny_pdg_cs | 11 | Statik analiz, indeks, genişletme |
| tinylink | 6 | Köprü, SDG, log deposu |
| tiny_rca | 10 | Birleştirme, yöntem eşleme |
| tinypraxis | 13 | Kısıtlı gezinti, aday seçimi |

**EK-2 — Ölçümlerin yeniden üretimi**

```bash
# Sembol indeksi ve graf
tiny-pdg-cs embed <proje> --store <depo> --reset \
  --provider openai --model <model> --base-url <url>

# Anlamsal arama ve genişletme
tiny-pdg-cs context "<sorgu>" --store <depo> --provider openai --model <model>

# Kaynak durumu (anlamsal kaynak çalıştı mı?)
tiny-rca capabilities

# Zincir uçtan uca
tinypraxis praxis_begin --trace <iz.json>
```

**EK-3 — Sembol sayılarının türetilmesi**

| Sayı | Türetme |
|---|---|
| 7.087 | Test kodu dâhil indekslenen sembol |
| 4.141 | Test kodu hariç; kurucu metotlar çıkarılmış |
| 2.087 | Filtre tarafından dışlanan test sembolü |
| 14.166 | $\|\mathcal{D}\| + \|\mathcal{K}\| + \|\mathcal{R}\|$ |
| 269 | $\| \{v : \deg(v) = 0\} \|$ |

**EK-4 — Kırılma noktaları kaydı**

3.7 bölümünde anlatılan dört olayın her biri için: beklenti, ölçüm, öğrenim ve tasarıma
eklenen kural. Bu kayıt, yöntemin niçet yazımla değil ölçümle sabitlendiğini gösterir.

---

*Bu metin taslaktır. Danışman onayı, İngilizce özet, özgeçmiş ve 5–6. bölümlerin
yazar tarafından yazılması tamamlanmadan teslim edilmemelidir.*