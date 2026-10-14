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
çalışmayı bırakmışken "başarılı" görünmüştür. Bunlardan biri, ölçülen 626.236 span'ın
hiçbirinde istisna tipi bulunmamasıydı: istisna kaydı hiçbir yerde yapılmıyordu.
Bu üç olay da tasarımın bir parçası olarak kayda geçirilmiştir.

Çalışmanın üçüncü temel bulgusu ölçülebilir bir kapsamadır. Dokuz alanın her biri ayrı
süreçte koşturulmuş, **546 fonksiyonel senaryonun tamamı geçmiştir**. Bu koşunun
ürettiği 187.562 span üzerinde ölçülen yol kataloğu 353 ayrı yol ve 33.520 yol adımı
içermektedir. İstisna taşıyan 37 span'ın **37'sinde** yol kimliği vardır (%100); hata
yanıtı veren 466 span'ın 465'i yola bağlıdır (%99,8). İz ile yol kataloğü arasındaki
bağlantının hata üreten isteklerde tam olarak kurulduğu bu ölçümle gösterilmiştir.

Son olarak, aynı veri kümesinde yalnızca 2 span 5xx durum kodu döndürmüş, buna
karşılık 353 ayrı yol kataloglanmıştır. Yalın hata oranı iki olayı gösterir; yol
kataloğu bu iki olayın geçtiği adımları ve bunların karşılık geldiği sınıf ve yöntemleri
verir. Tezin ölçülebilir katkısı bu ayrımı ölçülebilir kılmaktır.

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

- Şekil 3.1 — Dört katmanın veri akışı
- Şekil 3.2 — Bir sembolün indekslenme ve genişletilme yolu
- Çizelge 2.1 — Yaklaşımların aynı veri kümesindeki karşılaştırması
- Çizelge 2.2 — Katmanların sorumluluk ayrımı
- Çizelge 3.1 — Katmanların sorumluluk ve veri sahipliği dağılımı
- Çizelge 4.1 — Deney ortamı
- Çizelge 4.2 — Katman başına ölçülen durum
- Çizelge 4.3 — Graf genişletmesinin beş sorgudaki etkisi
- Çizelge 4.4 — Metin içeriği kararlarının ölçülen etkisi
- Çizelge 4.5 — Rapor doğruluğu düzeltmeleri ve ölçülen etkileri
- Çizelge 4.6 — Alan bazında fonksiyonel test sonucu
- Çizelge 4.7 — Yol kataloğu ölçümleri
- Çizelge 4.8 — Durum kodu, yol kapsaması ve istisna

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
üzerinde yürütülmüştür. Değerlendirme tek bir çözüm üzerinde yapılmıştır; başka dil ve
çözümlerde ölçüm yapılmamıştır.

Ölçülen dağıtımda servisler ortak bir veritabanını paylaşmakta ve birbirlerini
çağırmamaktadır; bu nedenle kendiliğinden oluşan izler tek servisli olmaktadır. Servis
bağımlılık grafiğinin kenar üretimi, kontrollü bir deneyle üretilen çapraz servis
izleri üzerinde doğrulanmıştır. Bu deney üretim verisi değildir ve sonuçların
değerlendirilmesinde böyle ayrılmıştır. Ayrıntı için bkz. Bölüm 4.6.4.

## 1.7. Tezin yapısı

Tez beş ana bölümden oluşur. **2. bölüm** kavramları tanımlar ve önceki çalışmaları
sunar. **3. bölüm** mimarî tasarımı ve matematiksel tanımlamaları içerir. **4. bölüm**
deneysel sonuçları verir. **5. bölüm** tartışma, sınırlılıklar ve sonucu içerir.

---

# 2. KURAMSAL ÇERÇEVE VE İLGİLİ LİTERATÜR

Bu bölüm, bu tezde kurulan araç zincirinin dayandığı kuramsal temeli kurar ve
her katmanın hangi alan literatüründe durduğunu gösterir. Bölüm, zincirin
türetim sırasına göre ilerler: önce çalışma zamanı gözlemlenebilirliği, sonra
statik koddaki bağımlılık yapıları, ardından ikisini birleştiren nedensel çıkarım
ve bu ikisini ajan biçiminde örgütleyen dil modeli araç zincirleri.

## 2.1. Gözlemlenebilirlik ve Dağıtık İzleme

### 2.1.1. İz kavramı

Dağıtık bir sistemde tek bir istek birden çok servise yayılır. Bu yayılımı
izlenebilir kılmak için her isteğe benzersiz bir tanımlayıcı atanır ve bu
tanımlayıcı, isteğin geçtiği her servis arasında aktarılır. Bu yayılım zincirine
**iz** (*trace*) denir; zincirin her adımına **span** denir.

Bir span $s$ aşağıdaki bilgiyi taşır:

$$s = \left\langle t_{start},\, t_{end},\, id,\, id_{parent},\, name,\, attributes \right\rangle \tag{2.1}$$

Burada $id_{parent}$, s'nin içinde bulunduğu üst span'in tanımlayıcısıdır. Bu
tek alan, iz ağacının hangi servise ait olduğunu belirler. Bir iz ağacı $T$,
span'ların $id_{parent}$ ilişkisiyle kurduğu ağaçtır:

$$T = \left\{ (id_{parent}(s),\, id(s)) \;\middle|\; s \in S \right\} \tag{2.2}$$

İz bağlamının yayılımı *W3C Trace Context* standardıyla tanımlanmıştır.
Standart, `traceparent` başlığında dört alan taşır: sürüm, iz tanımlayıcısı,
üst span tanımlayıcısı ve örnekleme bayrağı. Yayılım kuralı şudur: bir servis
istek aldığında üst span'in kimliğini okur, kendi span'ını bu kimliğin altına
açar ve kendi span kimliğini bir sonraki çağrıya aktarır.

Bu tasarımın tez açısından sonucu önemlidir: **bir istemci çağrısı ile onu
karşılayan sunucu çağrısı, aynı iz ağacında birbirinin altında değilse hiçbir
bağ kurulamaz.** Yani ölçüm boyunca kabul edilen koşul şudur:

$$\mathrm{parent}(c) = s \iff id_{parent}(c) = id(s) \tag{2.3}$$

Bu koşul sağlanmadığında, aynı izde bulunan iki kardeş span arasında hiçbir
servis bağımlılığı kenarı üretilemez. Bu, tezde ölçülmüş bir kısıttır.

### 2.1.2. İz–kod boşluğu

İz, isteğin **nereye gittiğini** gösterir. İsteğin neden oraya gittiğini, yani
hatanın **kodda** nerede oluştuğunu göstermez. Bu, literatürde *trace–code gap*
 olarak adlandirilen ve kök neden analizini zorlaştıran temel açıktır.

Sorunun kaynağı iki ayrı veri modelinin birbirinden habersiz olmasıdır. İz,
çalışma zamanında üretilen ve kısa ömürlü bir olaydır; kod ise derleme zamanında
yazılmış, uzun ömürlü bir yapıdır. Aralarında doğrudan bir işaretçi yoktur.
Bu nedenle araştırmacılar ya kök nedeni yalnızca çalışma zamanı verisinden
çıkarmaya (Li ve ark., 2018) ya da yalnızca statik kod analizine güvenmeye
(Blinowski ve ark., 2015) zorlanmıştır. Bu tezin konumu 2.8'de bu iki geleneği
birleştirme girişiminde tanımlanmıştır.

### 2.1.3. Gözlemlenebilirliğin maliyeti

Gözlemlenebilirlik bedelsiz değildir. Bir span üretmek, bağlamaktan kopyalama,
zaman damgalama ve diske yazma maliyetlerini getirir. Bu maliyetin karşılanmadığı
durumda, üretim sistemlerinde örnekleme (*sampling*) devreye girer ve izlerin
yalnızca bir bölümü kaydedilir. Oluşan boşluğa *observability gap* denir ve
bu boşluk, kök neden analizi için kaçınılmaz bir belirsizlik kaynağıdır.

Bu tezde ölçülen durum şudur: dokuz servisin dokuzu da telemeti üretmektedir
(ölçüm için Bkz. Bölüm 4.6.2). Yani bu çalışmada gözlemlenebilirlik boşluğu
kapandırılmış, maliyet nedeniyle bir boşluk bırakılmamıştır.

## 2.2. Program Bağımlılık Grafiği

### 2.2.1. Kontrol akış grafiğinden bağımlılık grafikine

Kontrol akış grafiği (CFG), bir metodun çalışma zamanında hangi blokları, hangi
sırayla ziyaret edebileceğini gösterir. Ancak CFG yeterli değildir: iki blok
 arasındaki tek bir aralıktan geçiş, kodun gerçekte neye bağlı olduğunu söylemez.
Örneğin bir hata, işareti hiç taşınmayan bir değişkenin değerine bağlı olabilir.

Ferrate, Ottenstein ve Warren (1988), bu eksikliği gidermek için **program
bağımlılık grafığını** (PDG) tanımlamıştır. PDG, CFG'yi üç ayrı kenar türüyle
zenginleştirir:

$$G_{pdg} = \left( V_{bb},\; E_{flow} \cup E_{ctrl} \cup E_{data} \right) \tag{2.4}$$

burada $V_{bb}$ temel blok kümesidir ve kenarlar şöyle tanımlanır:

- **Kontrol akışı** $E_{flow}$: $b_i \leadsto_{flow} b_j$, eğer $b_j$ bloğuna
  $b_i$'den veya $b_i$'nin sürekliliğinden ulaşılabiliyorsa.
- **Kontrol bağımlılığı** $E_{ctrl}$: $b_i \leadsto_{ctrl} b_j$, eğer $b_j$
  bloğu yalnızca $b_i$ bloğundaki bir koşulun sonucuna bağlı olarak yürütülüyorsa.
- **Veri bağımlılığı** $E_{data}$: $b_i \leadsto_{data} b_j$, eğer $b_j$ içinde
  tanımlanan bir değişken, $b_i$ içindeki bir tanımla geçersiz kılınıyorsa.

Program bağımlılığı grafiğinin klasik tanımı biraz daha kesindir. Bloklar
arasındaki gerçek bağımlılık şudur:

$$(u, v) \in E_{pdg} \iff \exists p \in P_{ctrl}(u) \cap P_{def}(v) \setminus P_{kill}(v) \tag{2.5}$$

burada $P_{ctrl}(u)$, $u$ bloğunun denetlediği yürütme yolları kümesidir;
$P_{def}(v)$ ve $P_{kill}(v)$ sırasıyla $v$'nin tanımlanan ve geçersiz kılınan
değişkenlerinin kümesidir. Denklem (2.5), $u$ bloğunun denetlediği bir yolda
$y$'nin geçersiz kılınmadan önce $v$ bloğunun bir tanımının bulunduğunu söyler.
Bu, *program slicing* ölçütünün de çekirdeğidir: bir $v$ bloğu, ölçüt $v$ ise
ondan etkilenen tüm blokları $v$'den geriye doğru izleyerek bulunur.

Bu tanımın doğruluğu üzerine sonraki çalışmalar önemli sonuçlar üretmiştir.
Weiss (1991), kontrol akış grafiği ile PDG'nin belirli bir dönüşüm altında
anlamsal olarak eşdeğer olduğunu göstermiştir; bu, PDG'nin CFG'den türetilir
olduğunu doğrular. Nagarajan ve ark. (2009), eşzamanlı programlarda PDG'nin
ayrı bir biçimini tanımlamış; Homerding ve ark. (2024) ise dönüşümün yönüne
bağlı olarak neden aynı sonucu vermediğini incelemiştir.

**Hammock blokları.** PDG üzerinde, benzerlik yüksek ancak davranışsal olarak
farklı olan iki bloğa **hammock** denir. Hammock blokları, yapısal olarak birbirine
benzeyen fakat farklı iş yapan kopyaları işaret eder. Bir metodun hammock
oranı, gövdesinin ne kadarının bu biçimde çürüdüğünü ölçer. Bu tezde ham
derleme çıktısı üzerinde hammock oranı ölçülmüştür.

### 2.2.2. PDG'nin program analizindeki yeri

PDG, kaba kuvvetle test üretimi, hata bulma ve program onarımı alanlarında
yaygın kullanılmıştır. Askarunisa ve ark. (2012), Java programlarında olasılıksal
bir PDG kurarak hata yerelleştirme yapan ilk çalışmalardan birini sunmuştur.
Zhang ve ark. (2023), PDG'yi sinir ağı tabanlı program onarımıyla birleştirerek
etkinlik filtresi olarak kullanmıştır. Son dönemde PDG, doğrudan dil modeli
istemlerinde de kullanılmaya başlamıştır: transformer tabanlı modellerin
program bağımlılığını öğrenip öğrenemediklerini ölçen çalışmalar (2607.23355),
PDG'nin bir referans model olarak hâlâ geçerli olduğunu göstermektedir.

Bu tezde PDG, kök neden adaylarını sıralamak için değil, aday metotların
**hangi bölgelerinin inceleneceğini** belirlemek için kullanılır. Bu, ilk
literatürden farklı bir kullanımımdır: PDG burada bir hata bulma motoru değil,
bir **uzay daraltma** aracıdır.

## 2.3. Çağrı Grafiği ve Depo Düzeyinde Kod Analizi

### 2.3.1. Çağrı grafiği

Çağrı grafiği, metodları düğüm, metotlar arasındaki çağrıları kenar olarak alan
yönlü graf:

$$G_{cg} = (V_{m},\; E_{cg}), \qquad E_{cg} = \{\, (u,v) \;\mid|\; u \text{ metodu } v \text{ metodunu çağırır} \,\} \tag{2.6}$$

Çağrı grafiği, metodun **etkisini** (kim onu çağırır) ve **kaynağını** (o neyi
çağırır) birlikte verir. İki işlev de bu tezde kullanılır: etki alanını
genişletmek için ters yön, patlama yarıçapını ölçmek için ileri yön.

### 2.3.2. Kayıp kenarlar ve sessizlik

Çağrı grafiğinin bilinen zayıflığı **eksik (missing) kenarlardır**: grafikte
olması gereken ama statik analizle bulunamayan çağrılar. Nedenleri çok çeşitlidir:
dinamik çağrı, yansıma, sanal metot çözümlemesi, kod üretimi. Bu eksiklik
Ammons ve ark. (2002) ile Bodden ve ark. (2010) tarafından ölçülmüştür.

Bu tezde eksikliklerin bir kısmı ölçülmüştür. Graf genişletme öncesi indekste
bulunan **yalıtılmış** (hiçbir kenarı olmayan) sembol sayısı 1.065'tir;
`references` ilişkisiyle genişletme sonrası bu sayı 269'a düşmüştür. Yani
genişletme kapsamı ikiye yakın artırmıştır. Kalan 269 yalıtılmış sembol,
çağrı grafiğinin kendi sınırıdır ve tezde sınır olarak raporlanmıştır.

### 2.3.3. Derinlik ilk arama ve patlama yarıçapı

Bir metottan erişilebilen işlevler, çağrı grafiğinde ileri yönlü geçişle
bulunur. Erişim kümesi, düğüm $m$'den $k$ adım uzaklıktaki düğümlerin birleşimidir:

$$\mathrm{Br}_k(m) = \bigcup_{i=0}^{k} N^{+i}(m) \tag{2.7}$$

burada $N^{+}(m)$, $m$'nin çağırdığı düğümler kümesidir. $k \to \infty$
alındığında (2.7), metottan erişilebilen tüm HTTP uçlarını verir; bu kümeye
**patlama yarıçapı** denir.

## 2.4. Gömme Tabanlı Anlamsal Arama

### 2.4.1. Kod gömme

Metinsel arama, eşleşme için harf dizisi özdeşliğine dayanır. Bu, bir kavramın
farklı ifadelerini yakalayamaz. Gömme tabanlı arama ise metni bir vektöre
dönüştürür ve benzerliği vektör uzayında ölçer.

Bir kod öğesi $x \in \mathcal{X}$ için gömme modeli $f$, metni bir vektöre
eşler:

$$f : \mathcal{X} \rightarrow \mathbb{R}^{d} \tag{2.8}$$

Bu tezde kullanılan model, boyutu $d = 768$ olan `nomic-embed-text` olmuştur.
Bu seçim, kod ile İngilizce metni ortak bir uzayda temsil edebilmesi ve
sorgunun doğal dilde yazılabilmesi nedeniyle yapılmıştır.

### 2.4.2. Simetriklik tezi

Kod aramasında metin–kod çiftleri **asimetriktir**: bir tarafta doğal dilde bir
sorgu, diğer tarafta bir kod parçası vardır. Kod arama literatürünün büyük
bölümü bu asimetriği bir noktada ihmal etmiştir. Kod arama
literatürü bu asimetriyi *simetriklik tezi* başlığı altında ele almış; iki tarafın
**birbirinin öğrentiği** iki yönlü bir model kurmanın başarımı artırdığını
göstermiştir.
Wu ve ark. (2022) aynı tespiti CodeSearchNet derlemi üzerinde yeniden üretmiştir.

Ancak bu gelenekte asimetriyi modelin *içinde* çözen yaklaşımlar da vardır.
Gu ve ark. (2021), kodu birden çok alt dil biriminin birleşimi olarak temsil
etmiştir; Zeng ve ark. (2021) ise akış grafını doğrudan gömmüştür. Bu iki
yaklaşım, asimetrinin metin tarafında değil **kod tarafında** çözülebileceğini
göstermektedir; bu, kod ve metin gömme boyutlarının eşit tutulmasını
gerektirmez.

### 2.4.3. Benzerlik ölçüsü

İki gömme arasındaki benzerlik, yaygın olarak kosinüs benzerliği ile ölçülür:

$$\mathrm{sim}(x, y) = \cos(\theta) = \frac{f(x) \cdot f(y)}{\lVert f(x) \rVert_2 \;\lVert f(y) \rVert_2} \tag{2.9}$$

Bu ölçü, vektörlerin yönlerini karşılaştırır, boyutlarını değil. Bu yüzden
farklı uzunluktaki kod parçaları karşılaştırılabilir hâle gelir.

Kosinüs benzerliğinin kod aramasında bir zayıflığı vardır: gömmeler yüksek
boyutlu uzayda **izotropik** olmadığında benzerlik değerleri dar bir aralığa
sıkışır ve ayırt edicilik düşer. Diera ve ark. (2024), bu etkiyi ölçmüş ve
izotropik düzeltmenin arama başarımını artırdığını göstermiştir.

Sorgu $q$ için aday kümesi, kosinüs benzerliğine göre sıralanır ve eşik $\tau$
üzerinden filtrelenir:

$$C(q) = \Big\{\, s \in S \;\big|\; \mathrm{sim}(q, s) \ge \tau \,\Big\}_{\text{azalan sıra}} \tag{2.10}$$

### 2.4.4. Depo düzeyinde kod araması

Metin araması tek bir dosyayla sınırlıdır; oysa bir kavram genellikle birden
fazla dosyaya yayılır. Bu sorunu çözmek için son dönem çalışmaları iki yaklaşım
izlemiştir.

**Çok modlu temsil.** Ham metin, sözdizim ağacı, kontrol akış grafiği ve veri
akışı gibi birden çok görünüm birlikte temsil edilir. Gu ve ark. (2021)
sözdizim ağacını, Zeng ve ark. (2021) akış grafını kullanmıştır. Bu yaklaşımın
zayıflığı, ağaç derlemesinin derleme ortamına bağlı olmasıdır.

**Grafla genişletme.** Semantik olarak yakın bulunan semboller bir **tohum**
kümesi oluşturur; bu küme çağrı grafiği üzerinde genişletilir. Jain ve ark.
(2024) LLM ajanlarının semantik kod aramasını iyileştirdiğini göstermiş,
RANGER (Shah vd., 2025) ise graf destekli genişletmeyi depo düzeyinde bir
ajan mimarisine yerleştirmiştir. Bu yaklaşımın statik karşılığı da vardır:
Veenendaal ve Daly (2016), tanım yerine **çağrı noktası** analizinin çağrı
grafiği üretiminde daha kesin sonuç verdiğini göstermiştir.

Bu tezde ikinci yaklaşım benimsenmiştir. Genişletme kuralı şudur:

$$R_q = C(q) \;\cup\; N^{+}(C(q)) \;\cup\; N^{-}(C(q)) \tag{2.11}$$

Simgem eşiklenmiş aday kümesidir, $N^{+}$ ve $N^{-}$ sırasıyla çağırdığı ve
çağıran düğümlerdir. (2.11), anlamsal benzerliğin yapısal erişilebilirlikle
birleştirildiği ilk yerdir; tezin asıl önerisi bu birleşimdir.

## 2.5. Servis Bağımlılık Grafiği

### 2.5.1. Tanım

Servis bağımlılık grafıği (SDG), servisleri düğüm, servisler arası çağrıları
kenar olarak alan yönlü graf:

$$G_{sdg} = (S,\; E_{sdg}), \qquad E_{sdg} = \{\,(a,b) \;\mid|\; a \text{ servisi } b \text{ servisini çağırır} \,\} \tag{2.12}$$

Mikro-mimari keşfi literatürü bu grafiği çıkarma sorununa odaklanmıştır. Neovius
ve ark. (2016), servis bağımlılığının bir kuramını kurmuştur. Landau ve ark. (2025) ise
dağıtık sistemlerde servis bağımlılığı keşfini geriye dönük olarak ele almış,
yöntemin endüstriyel ortamda geri kazanım için kullanılabileceğini göstermiştir.

### 2.5.2. İzlerden SDG çıkarımı

SDG doğrudan ölçülemez; izlerden çıkarılır. Çıkarımın temel kuralı (2.3)'te
verilmiştir: bir kenar ancak istemci–sunucu **çifti** oluştuğunda üretilir.

$$\mathrm{edge}(a, b) \iff \exists\, c \in S_a,\; s \in S_b :\; id_{parent}(c) = id(s) \tag{2.13}$$

Bu kural, yalnızca aynı izde bulunmaya dayanan zayıf bir eşleştirmeyi bilinçli
olarak reddeder. Bu tezde ölçülen sonuç şudur: bu kural uygulanmadan önce
kenarların tamamı boş kalıyordu; uygulandıktan sonra `Agency.API → Billing.API`
kenarı oluştu. Kural aynı zamanda yalancı pozitifleri de azaltır, çünkü iki
servisin aynı izde görünmesi onlar arasında gerçek bir çağrı olduğunu kanıtlamaz.

## 2.6. Nedensel Çıkarım ve Kök Neden Analizi

### 2.6.1. Nedensellik

Bir gözlemsel nedensellik problemi şu biçimdedir: gözlenen değişkenler arasındaki
nedensel ilişki nedir? Gözlemsel veriden nedensel yapı çıkarımına **nedensel
keşif** denir.

Bir nedensel yapı, doğrudan nedensellik (doğrudan etki), ortak neden ve
çakışma gibi nedensel ilişki türlerini temsil eder. Bir yapının doğruluğu üç
varsayıma dayanır: **kardinalite**, **doğrusallık** (determinizm) ve **adalet**
(müdahale edilebilirlik). Bu varsayımlardan biri ihlal edilirse çıkarılan yapı
gerçeği yansıtmaz. Doğrusallığın ihlali, dağıtık sistemlerde en sık görülen
gürültü kaynağıdır.

### 2.6.2. Nedensel keşif yöntemleri

Nedensel keşif algoritmaları üç aileye ayrılır.

**Kontrollü deneylerle.** Müdahale yapılarak nedensellik doğrudan sınanır.
Algoritmik olarak güçlüdür, ancak üretim sistemlerinde genellikle uygulanamaz.

**Gözlemsel.** Ortak neden ve dışsallık (dış nedensellik) varsayımlarıyla
istatistiksel ilişkilerden çıkarım yapar. Zamansal öncelik (temporal
precedence) varsayımı temeldir: neden, etkiden önce gelmelidir.

**Zamansal öncelik çıkarımı.** Zamansal nedensel çıkarım (TCM), nedenselliği
doğrudan nedensel grafla değil, değişkenler arası **gecikme** ilişkisiyle
tahmin eder. Gözlemsel nedensal çıkarımın en zor kısmı olan nedensel etki
yönünün ayrımını, gecikmenin yönünden çözer. Zaman serisi nedensel keşfine
yönelik sentetik kıyaslama veri kümeleri bu alandaki ölçüm standardını
belirlemiştir.

Bu tezde nedensel çıkarım paketi kullanılmıştır (`causal-learn`). Bu paket,
zamansal öncelik varsayımını açıkça belirterek, nedensel yönün belirsiz
olduğu durumlarda çıkarımı durdurur; sessizce bir yön seçmez.

### 2.6.3. Dağıtık sistemlerde kök neden analizi

Bu literatürün çekirdek sorusu şudur: gözlenen bir arızanın kök nedeni nedir?
Çalışmalar iki ana aileye ayrılır.

**İstatistiksel ve nedensel çıkarım yaklaşımları.** Yalnızca ölçüm verisine
dayanır. Zhang ve Fang (2025) nedensellik farkındalıklı dinamik kök neden analizi
önermiştir. Lin ve Chang (2024) sinir Granger nedensel keşfini
kullanmaktadır. Pham ve Ha (2024) çok değişkenli Bayesçi bir yaklaşım
(BARO) kurmuştur. Pham ve Ha (2024a), bu yöntemlerin mikroservislerdeki
etkinliğini dokuz nedensel keşif ve yirmi bir kök neden analizi yöntemiyle
karşılaştırmış; **nedensel çıkarımın bugünkü düzeyinin sınırlı olduğunu**
ölçerek göstermiştir. Bu, bu tez için önemli bir olgudur: ölçülen bir yaklaşımın
da kendi sınırı vardır ve bu sınır raporlanmıştır.

**İz tabanlı yaklaşımlar.** Kök neden, iz üzerindeki hata işaretlerinin
yayılımından çıkarılır. Li ve ark. (2018) bu dalın kurucu çalışmasıdır; her
servisin gecikme dağılımını karşılaştırarak anormal olanı işaret eder. Wang ve
ark. (2021) olay grafiği üzerinden endüstriyel ortamda kök neden çıkarımı
yapmıştır. Zhao ve ark. (2024) çok modlu sistemlerde nedensel hipergraf
kullanmıştır.

Son dönemde iz tabanlı çalışmalar dil modellerine yönelmiştir. Birleştirilmiş
koşullu öğrenme ile kök neden analizi (2511.17566), spekülatif kök neden analizi
(2601.02736) ve LLM tabanlı mikrokök neden ajanları (2509.15635) bu hattın
örnekleridir.

### 2.6.4. Kaba kuvvet kök neden adayları

Son yıllarda kök neden adaylarını sıralama problemi bir **öğrenme problemine**
dönüştürülmüştür: adaylar bir küme olarak üretilir, bir **aday sıralayıcı** ile
sıralanır. Sıralayıcının amacı, gerçek kök nedeni ilk sıralamaya taşımaktır.
Bir değerlendirme ölçütü olarak:

$$\text{Recall}@k = \frac{\left| \, C \cap R_k \,\right|}{\left| C \,\right|} \tag{2.14}$$

burada $C$ gerçek kök neden kümesi, $R_k$ sıralayıcının ilk $k$ adayıdır.

Bu tezde aday üretimi sıralayıcı öğrenmeden, **graf kısıtıyla** yapılır. Kısıt,
§2.8'de tanımlanan bütçeli gezinti ile uygulanır. Bu, makine öğrenmesi tabanlı
sıralayıcılara bir alternatiftir: eğitim verisi gerektirmez ve açıklanabilirdir.

## 2.7. Dil Modeli Ajanları ve Araç Zincirleri

### 2.7.1. Akıl ve eylemi birleştirme

Yao ve ark. (2022), büyük dil modellerinde **düşünme** ve **eyleme**in
dönüşümlü kullanılmasını öneren ReAct çerçevesini sunmuştur. Çerçeve, dil
modelinin bir yandan iz (thought) üretmesine, diğer yandan dış araç (action)
çağırmasına izin verir; iki adım birbirinin sonucuna göre sırayla ilerler:

$$ \underbrace{a_t}_{\text{eylem}} = f\big(\, s_t,\, \underbrace{o_t}_{\text{gözlem}} \,\big),
\qquad s_{t+1} = s_t \Vert a_t \Vert o_t \tag{2.15}$$

Yao ve ark. (2022) bu biçimin akıl yürütme ve etkileşim görevlerinde ReAct
temelli basit istemlere göre belirgin bir üstünlük sağladığını göstermiştir.

PRAXIS (Cui ve ark., 2025), bu çerçeveyi kök neden analizine uyarlamıştır ve
bu tezin çerçevesini doğrudan belirlemiştir. PRAXIS, ReAct tabanlı bir temel
hattın iki graf üzerinde — servis bağımlılık grafiği ve hammock bloklu program
bağımlılık grafıği — yapılandırılmış bir gezinti yürütüğünü ve kök neden
doğruluğunda 6,3 kata kadar, jeton tüketiminde 5,3 kata kadar iyileşme
sağladığını raporlamıştır.

### 2.7.2. Araç kullanımı ve arayüzün rolü

Ajan tarafından çağrılan araçların sayısı ve niteliği arttıkça, arayüz
tasarımının başarı üzerindeki etkisi tartışılmaya başlanmıştır. Xu ve ark.
(2026), tek araç çağrısından çok araç çağrısına geçişin evrimini
sistematize etmiştir. Özellikle denetimli karşılaştırmalarda **iskele
(scaffolding)** tasarımının arayüzden daha belirleyici olduğu bulunmuştur.
Bu, bu tez için bir uyarıdır: araç sayısını artırmak tek başına başarı
getirmez; gezintiye hangi aracın hangi aşamada açılacağı, araç setinden daha
önemlidir.

### 2.7.3. Model Context Protocol

Model Context Protocol (MCP), dil modeli uygulamalarının araçlara bağlanmasını
standartlaştıran bir açık protokoldür. Protokol, bir istemci ile bir veya
dokuz sunucu arasında, araç listeleme (`tools/list`) ve araç çağırma
(`tools/call`) işlemlerini tanımlar.

Protokolün bu tez için önemi mimaridir: araçlar birbirinden bağımsız süreçler
hâlinde çalışır ve yalnızca standart bir arayüz üzerinden konuşur. Bu, katmanların
bağımsız geliştirilmesini ve ayrı ayrı doğrulanmasını mümkün kılar. Bu tezde
her katman ayrı bir MCP sunucusu olarak derlenmiş ve ayrı ayrı test edilmiştir.

### 2.7.4. Bütçeli ve kısıtlı gezinti

Serbest bir ajan gezintisi bir noktada erken durabilir: kuyruk boş görülebilir,
bütçe tükenebilir ya da araç zinciri sonsuz döngüye girebilir. PRAXIS bu
sorunu bir **kapanma koşulu** ile çözer: gezinti ancak kuyruk boşaldığında ve
her varlık bir role atandığında kapanır.

$$Q = \varnothing \;\wedge\; \forall v \in S:\; \rho(v) \neq \bot \tag{2.16}$$

Bu biçim bir sınırlamadan çok bir **garanti**dir: hiçbir varlık rapor
üretilmeden önce yargılanmış olur. Bu tezde bu koşul doğrudan uygulanmış ve
kuyruk boşalana kadar rapor üretilmemesi ölçülerek doğrulanmıştır.

### 2.7.5. RAG ve bilgi geri getirme

Bu tezde dil modeli çağrısı yapılmaz; tüm katmanlar deterministik olarak
çalışır. Yine de bağlam üretimi, RAG literatüründeki etkin geri getirme
(konumlandırma) fikrinden yararlanır: ilgili bölge önce daraltılır, sonra
işlenir. Asıl fark, geri getirilen metin yerine bir **program bağımlılığı
grafı bölgesi** getirilmesidir.

## 2.8. Bu Tezin Konumu

Yukarıdaki literatür üç geleneği birbirinden ayrı geliştirmiştir.

1. **Yalnızca gözlemlenebilirlik** çalışmaları hatanın *neyi* etkilediğini
   bulur, *nerede* olduğunu bulamaz (Li ve ark., 2018; Wang ve ark., 2021).
2. **Yalnızca statik analiz** çalışmaları hatanın nerede olabileceğini söyler,
   o anda gerçekten olup olmadığını söyleyemez (Ammons ve ark., 2002;
   Bodden ve ark., 2010).
3. **PRAXIS** (Cui ve ark., 2025) bu ikisini ajan biçiminde birleştirir ve
   etkinliğini raporlar; ancak gezintinin nasıl sonlanacağı bir prosedür
   düzeyinde tanımlanmaz.

Bu tezin katkısı, birleşimi bir **açık kapanma koşuluyla** tanımlamaktır:
(2.16) eşitliği, gezintinin hiçbir varlık atlanmadan sonlanmasını garanti eder.
Ayrıca aday üretimi öğrenilmiş bir sıralayıcıya değil, graf kısıtına
dayandırıldığı için eğitim verisi gerektirmez ve her adımı gerekçelendirilebilir.

Tezin somut ölçümleri, bu konumun deneysel karşılığıdır ve Bölüm 4'te
sunulmuştur.

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

Tablodaki sayılar şu iddiayı ölçerek desteklemektedir: aynı veri kümesinde
yalnızca **2 span** 5xx durum kodu döndürmüş, buna karşılık **353 ayrı yol**
kataloglanmıştır. Hata oranı tek başına iki olayı göstermekte, yol kataloğu ise
bu iki olayın hangi adımlardan geçtiğini ve o adımların hangi sınıf ve yönteme
karşılık geldiğini vermektedir.

**Çizelge 2.2 — Katmanların sorumluluk ayrımı**

| Yetenek | tiny-pdg-cs | tinylink | tiny-rca | tinypraxis |
|---|---|---|---|---|
| Statik sembol bulma | 4.141 sembol | — | — | — |
| Çağrı grafiği | 14.166 kenar | — | — | — |
| Servis bağımlılık grafiği | — | var | — | var |
| Hata anını yola bağlama | — | var | var | var |
| Kök neden gezintisi | — | — | — | 4 aşama |
| Dil kapsamı | C# | C# + OTel | C# + OTel | çok dilli |

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

**Şekil 3.1 — Dört katmanın veri akışı:**

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

$$
\mathcal{R}(c_i) = \{ r \in \{\text{açık}, \text{sözcüksel}, \text{anlamsal}\} \}
\tag{3.1}$$

**Açık eşleşme:**

$$
\text{açık}(c_i) \iff \mathrm{alt}(d_i) \subseteq \mathrm{alt}(q)
\tag{3.2}$$

burada $\mathrm{alt}(\cdot)$ küçük harfe indirgemedir. Sorgu, sembolün *kendi adını*
anlatıyorsa eşleşme vardır.

**Sözcüksel eşleşme:**

$$
\text{sözcüksel}(c_i) \iff \left| T(q) \cap T(\tau_i) \right| > 0
\tag{3.3}$$

burada $T(\cdot)$ token çıkarımıdır ve camelCase bölme uygular.

**Anlamsal eşleşme:**

$$\text{anlamsal}(c_i) \iff d_i \in K(q)$$

**Önemli tasarım kararı — ortalamak yok.** Bu üç sayının ortalanması **bilinçli olarak
yapılmamıştır.** Ortalama, okunması gereken şeyi gizlerdi: "iki kaynak buldu, üçüncüsü
bulamadı" bilgisi. Bir cevabın güvenilirliği, kaynakların birbirini doğrulamasıyla
ölçülür, puanla değil.

**Sıralama.** Kaynaklar güçlülüğe göre sıralanır ve liste bu sırayı okur:

$$
\text{sıra}(\text{açık}) = 3 > \text{sıra}(\text{sözcüksel}) = 2 > \text{sıra}(\text{anlamsal}) = 1 > \text{sıra}(\text{genişletilmiş}) = 0
\tag{3.4}$$

Genişletilmiş adaylar sıfır alır: çünkü onlar sorguyla **hiç karşılaştırılmamıştır.**

### 3.3.3. Graf ve komşuluk

Kod grafiği:

$$
\mathcal{G} = (V, E), \quad V = \mathcal{C} \cup \mathcal{T}, \quad E = \mathcal{D} \cup \mathcal{K} \cup \mathcal{R}
\tag{3.5}$$

| Küme | İlişki | Anlamı |
|---|---|---|
| $\mathcal{D}$ | (tip, metot) | kapsama |
| $\mathcal{K}$ | (çağıran, çağrılan) | çağrı |
| $\mathcal{R}$ | (kullanıcı, kullanılan) | tip kullanımı |

**Bir adım komşuluk kümesi:**

$$
\mathcal{N}_1(v) = \{ w \in V \;:\; (v, w) \in E \;\lor\; (w, v) \in E \}
\tag{3.6}$$

İki yön birleştirilir, çünkü "bu kaydı kim kullanıyor" sorusu, "bu kayıt kimi kullanıyor"
sorusu kadar önemlidir.

**Yalıtılmışlık oranı:**

$$I = \frac{|\{ v \in V : \deg(v) = 0 \}|}{|V|}$$

$\deg(v) = |\mathcal{N}_1(v)|$. Bu oran, grafiğin ne kadar bilgi taşıdığının en kısa göstergesidir.

### 3.3.4. Genişletme

$$
\mathcal{E}_1(c) = \{ w \in \mathcal{N}_1(c) \;:\; w \notin \mathcal{R}_{\text{bulunan}} \}
\tag{3.7}$$

Genişletme yalnızca **hiçbir kaynak tarafından bulunmamış** sembolleri ekler. Zaten
bulunmuş bir adayı yeniden eklemek bilgi değildir.

**Sınırlar:**

$$
\left| \text{tohum} \right| \leq 3, \qquad \left| \mathcal{E}_1 \right| \leq 5
\tag{3.8}$$

Sınırlar keyfî değildir: üç tohum ve beş sonuç, listenin faydalı olmaktan çıktığı nokta
ölçülerek belirlenmiştir.

**Çıktının sıralamadaki yeri.** Genişletilmiş adaylar listenin **sonuna eklenir**;
mevcut sıralamayı değiştirmezler. Bu, 4.7'deki sınırlılığın nedenidir.

### 3.3.5. Güven ölçütü

$$
\text{güven} = \begin{cases} \text{yüksek} & |\mathcal{R}_{\text{bulunan}}| \geq 2 \\ \text{düşük} & \text{ aksi halde} \end{cases}
\tag{3.9}$$

$$
\text{doğrulanan} = \left| \{ c \in \mathcal{C} : |\mathcal{R}(c)| \geq 2 \} \right|
\tag{3.10}$$

Bu tanım, **kaç kaynağın bulduğunu** ölçer. Doğrulamanın yokluğu "bu bir tahmindir"
demektir.

## 3.4. Anlamsal indeks: tasarım ve sınır

### 3.4.1. Ne indekslenir

Her $c_i$ için indekslenen metin, 2.3.1'deki $\phi$'nin girdisidir:

$$
\Phi(c_i) = d_i \;\Vert\; f_i \;\Vert\; \tau_i \;\Vert\; \Gamma_i
\tag{3.11}$$

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

**Şekil 3.2 — Bir sembolün indekslenme ve genişletilme yolu**

```
  Kod öğesi c_i
        │
        ├─→ Φ(c_i) = d_i ‖ f_i ‖ τ_i ‖ Γ_i       (indeksleme metni)
        │        │
        │        └─→ f(Φ(c_i)) ∈ ℝ^768           (gömme)
        │
  Sorgu q ──→ sim(q, c) = cos θ                  (2.9)
        │        │
        │        └─→ sim ≥ τ ise C(q)'e ekle     (2.10)
        │
        └─→ Expand(C(q)) = C(q) ∪ N⁺ ∪ N⁻         (2.11)
                 │
                 ├─→ N⁺: c'nin çağırdığı semboller
                 └─→ N⁻: c'yi çağıran semboller
```

Genişletme, anlamsal eşleşmeyi yapısal erişilebilirlikle birleştiren tek
adımdır. Ölçülen etkisi Bölüm 4.3'te verilmiştir.

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

$$
\mathcal{R}_1 = \{ (\kappa(c_i), T) : T \in \text{uygulanan tip adları} \} \cup \{ (\kappa(c_i), \text{konstrüktör}, \tau) : \tau \in \mathcal{C}_{\text{konstrüktör}} \}
\tag{3.12}$$

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

$$
\text{durum}(r) \in \{\text{çalıştı},\; \text{çalışmadı: } neden\}
\tag{3.13}$$

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

### 3.7.5. Ölçülemeyen katmanın ölçülmüş gibi raporlanması

Dört katmanlı zincirin üçüncüsü olan telemeti okuma katmanı, çalışma boyunca
"bağlı" görünüyordu. Bu görünüm bir ölçümle yanlışlanmıştır.

**Ölçülen.** Telemeti deposunda biriken 626.236 span incelendi. `exception_type`
alanının dolu olduğu span sayısı **sıfırdı.** Alanı dolduran okuyucu doğru
çalışıyordu, ancak alanı dolduracak olan kayıt hiçbir yerde yapılmıyordu. İstisna
kaydı çağrısı kod tabanının hiçbir noktasından yapılmıyordu ve telemeti ara
katmanında `try`/`finally` bulunmasına rağmen `catch` bulunmuyordu.

**Düzeltilen.** İstisna, span'a `exception.type`, `exception.message` ve
`exception.stacktrace` etiketleriyle yazılmakta ve span durumu hata olarak
işaretlenmektedir. Düzeltme sonrası fonksiyonel koşuda **37 istisnalı span**
kaydedilmiştir.

**Doğrulama.** Söz konusu 37 span'ın **37'sinde** yol kimliği vardır (%100). Bu,
kök neden aracının hata anını adlandırabilmesi için gereken önkoşulun sağlandığını
gösterir. Aynı oran düzeltme öncesinde %0'dı.

**Öğrenilen.** Bir katmanın "bağlı" görünmesi, o katmanın veri ürettiği anlamına
gelmez. Depoda veri bulunması ile o verinin **doğru alanlara** yazılması ayrı
iddialardır. İkincisi ancak alan bazında sayılarak doğrulanabilir.

**Tasarıma eklenen.** Telemeti katmanının kabul ölçütü, "araç yanıt veriyor"
değil, "hata üreten isteklerde istisna tipi dolu span üretiyor" olarak
tanımlanmıştır. Bu ölçüt 4.6.3'te raporlanmaktadır.

### 3.7.6. Başarı olarak raporlanan hiçbir şey

Fonksiyonel test koşusunda ölçülen iki hata, doğrulama aracının kendisine ilişkindir.

**Ölçülen — derleme başarısızlığı yeşil sayıldı.** İlk koşu 25 senaryo beklenirken
**0 senaryo** yürüttü ve "geçti" bildirdi. Nedeni, hedefin derlenmemesiydi;
derleme çıktısı farklı bir kullanıcıya ait önbellek dosyaları tarafından
engelleniyordu.

**Ölçülen — süreç durumu yanlış gösterildi.** API'leri kaldıran betik, süreç
tanımlayıcı dosyasına yazamadığında hata veriyor, ardından tanımlayıcıyı diskteki
eski dosyadan okuyordu. Bildirilen tanımlayıcı ölü bir sürece aitti; gerçek
dinleyiciyle uyuşmuyordu. Kapatma komutu bu nedenle çalışan servisleri
durduramıyordu.

**Öğrenilen.** Bir doğrulama aracının kendisi hatalıysa, geçtiğini söylediği hiçbir
test güvenilmez değildir. Sıfır senaryo yürütülmüş bir koşu "geçti" değil, "koşulmadı"
dır. Bu ayrım raporlanmasaydı, 546 senaryoluk sonuç da güvenilmez olurdu.

**Tasarıma eklenen.** Fonksiyonel test çıktısı yalnızca geçen senaryo sayısıyla
değil, **yürütülen senaryo sayısıyla** birlikte raporlanmaktadır (Çizelge 4.6).

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

**Çizelge 4.4 — Metin içeriği kararlarının ölçülen etkisi**

| Karar | Ölçüm | Sonuç |
|---|---|---|
| Kurucu metotlar indekslenmesin | 494 → 0 kurucu sembolü | İlk 12 sonucun 3'ü kurucuydu; kaldırıldı |
| Tür bilgisi metinden çıkarılsın | Sorgu "list the members": tip → metot | 1 iyileşme, 0 gerileme |

## 4.5. Rapor doğruluğu ölçümleri

Üç gözlem ve düzeltmeleri:

**Çizelge 4.5 — Rapor doğruluğu düzeltmeleri ve ölçülen etkileri**

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

**Çizelge 4.6 — Alan bazında fonksiyonel test sonucu**

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

**Çizelge 4.7 — Yol kataloğu ölçümleri**

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

**Çizelge 4.8 — Durum kodu, yol kapsaması ve istisna**

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

### 4.6.4. Servis bağımlılık grafiğinin kontrollü deneyle doğrulanması

Ölçülen dağıtımda servisler ortak bir veritabanını paylaşmakta ve birbirlerini
çağırmamaktadır. Bu nedenle kendiliğinden oluşan izlerde her kayıt tek servisli
kalmaktadır ve servis bağımlılık grafiğinin kenar üretimi kendiliğinden
gözlemlenemez.

Bu nedenle kenar üretimi **kontrollü bir deneyle** doğrulanmıştır. Deneyde iki
çalışan servis arasına `traceparent` başlığı ile gerçek bir istek gönderilmiş, iz
üzerinde istemci ve sunucu span çifti oluşturulmuş ve kuralın (2.13) uygulanıp
uygulanmadığı ölçülmüştür.

| Durum | Kural (2.13) | Ölçülen sonuç |
|---|---|---|
| Kural uygulanmadan | eşleşme yok | Tüm kenarlar boş kaldı |
| Kural uygulandıktan sonra | istemci–sunucu çifti | `Agency.API → Billing.API` kenarı oluştu |

Bu bulgu, 2.5.2'de ileri sürülen kuralın deneysel karşılığıdır: iki span aynı izde
bulunsa bile kardeş ise kenar üretilmez; kenar ancak istemci–sunucu çifti
oluştuğunda üretilir.

Aynı deney kapsamında kök neden gezintisi de yürütülmüştür. `praxis_begin`,
`praxis_select` ve `praxis_next` adımları üç turda tamamlanmış, kuyruğun boş
olmadığı ve kapatma koşulunun (2.16) gezinti bitmeden sağlanmadığı gözlenmiştir.

> **Bu deney üretim verisi değildir.** İzler bilinçli olarak oluşturulmuştur.
> Sonuçların değerlendirilmesinde bu ayrım korunmuş, deney ayrı bir başlıkta
> ayrılmıştır. Kontrolsüz dağıtımda gözlenen tekil servis oranı 1,00'dir
> (bkz. Bölüm 4.7).

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
| S5 | Sıralamayı değiştirmeyen genişletme (S2) bir eksiklik mi, özellik mi? | 4.7'de sınır olarak kayıtlı |

## 5.2. Sınırın kendisi hakkında konuşulması gerekenler

**Veri kümeleri tek servisli (4.7/S1).** Bu sınır yalnızca servis bağımlılık
grafiğini etkiler. Statik analiz, sembol indeksi, yol kataloğu ve hata–yol
bağlantısı bu sınırdan etkilenmez ve ölçülmüştür. Yazarın tartışması şu soruya
odaklanmalıdır: *kontrollü deneyle doğrulanmış bir katman, kendiliğinden oluşan veriyle
de doğrulanmış sayılabilir mi?* Ölçülen cevap: kenar üretimi doğrudur, ancak kenar
dağılımı üretimde farklı olabilir.

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
2. **Tasarım katkısı:** Rapor dürüstlüğü (Bölüm 3.6) ve kaynak durumu kavramı, araç zincirleri
   için bir sözleşme önerir.
3. **Ölçümsel katkı:** Genişletmenin ölçülen etkisi (4.3) ve yalıtılmışlık oranının
   %25,7'den %6,5'e düşmesi.

## 5.4. Gelecek çalışmalar için çıkarılan sonuçlar

Tez boyunca ölçümle desteklenen ve henüz yapılmamış işler:

| Öneri | Dayanak |
|---|---|
| Çok servisli iz üreten yük testi | 4.7/S1 |
| Genişletme gürültüsünün sayılması | 5.1/S4 |
| Graf mesafesine dayalı puanlama kuralı | 4.6/S4 |
| Her sunucuya "çalışmayı bırak" bütünlük testi | 3.7'nin üç olayı |
| İkinci projede tip kullanım kenarının doğrulanması | 5.1/S3 |

---

# 6. SONUÇ

> **Yazar notu — bu bölüm de aynı kapsamdadır.** Aşağıdaki iskelet, sonucun hangi
> ölçülmüş bulgulara dayanabileceğini gösterir. Yazımı yazara aittir.

## 6.1. SONUÇUN DAYANMASI GEREKEN BULGULAR

Bu tezin sonucu, aşağıdaki yedi **ölçülmüş** bulguya dayanabilir. Bunların her biri
4. bölümde ve EK-2'de yeniden üretilebilir biçimde kayıtlıdır:

| # | Bulgu | Sayısal karşılığı |
|---|---|---|
| B1 | Katmanlar arası genişletme, yalnızca graf üzerinden ulaşılan metotlar üretir | `AddCreditCommandHandler` ilk 10 sonuçta yoktu |
| B2 | Tip kullanım kenarı grafiğin kapsamını ölçülebilir biçimde genişletir | $I$: %25,7 → %6,5 |
| B3 | Anlamsal aramanın isim/eylem eğilimi bir model kusuru değil, uzayın simetrikliğinin sonucudur | 5 sorgunun 3'ünde ilk sonuç tip |
| B4 | Bir sistemin kendini doğru tanıtması, iş yapmasından daha zordur | 6 kırılma noktası (3.7) |
| B5 | Yol kataloğu, iz ile kodu birbirine bağlayan son halkadır | 353 yol, 33.520 adım, 95 adım/yol |
| B6 | Hata üreten isteklerde yol kapsaması eksiksizdir | 37 istisnalı span'ın 37'si yolludur (%100) |
| B7 | Zincir uçtan uca çalışır | 9 alan, 546 senaryo, tamamı geçti |

## 6.2. Sonucun SÖZLEMEZLEMESİ GEREKEN NOKTALAR

Yazarın sonucu **kaçınmaması** gereken dört nokta vardır:

1. **Servis bağımlılık grafiği kendiliğinden oluşan veriyle doğrulanamamıştır.**
   Ölçülen dağıtımda servisler birbirini çağırmamaktadır. Kenar üretimi, 4.6.4'te
   açıkça belirtildiği gibi kontrollü deneyle doğrulanmıştır. Sonuç, bu katmanın
   kendiliğinden oluşan trafikte çalıştığını söylememelidir.

2. **Kök neden doğruluğu ölçülmemiştir.** 546 senaryo, aracın çalıştığını ve veri
   zincirinin bütünlüğünü gösterir; bir olayın *doğru* kök neden olarak
   adlandırıldığını göstermez. Doğruluk için işaretli bir olay kümesi gerekir ve
   bu küme bu çalışmada oluşturulmamıştır. Tez, doğruluğu değil **tutarlılığı**
   ölçmektedir.

3. **Genişletme sıralamayı değiştirmez.** Katkı "aday kümesini genişletmektir",
   "doğru sıralamaktır" değildir. Genişletilmiş adaylar listenin sonuna eklenir.

4. **Ölçüm tek projede yapılmıştır.** Genellenebilirlik iddiası, ikinci bir projede
   doğrulanmadan kurulamaz.

---

# KAYNAKÇA

> **Doğrulama notu.** Aşağıdaki arXiv girdilerinin künyeleri (yazar, yıl, başlık,
> kimlik) `paper` sunucusundan **doğrulanmıştır**. DOI ve hakemli yayın yeri
> alanları arXiv ön baskılarında bulunmadığı için boştur; teslimden önce
> tamamlanmalıdır. Klasik literatür girdileri (Ferrate vd., Li vd. 2018 gibi)
> metin içinde adı geçmekte, ancak künyeleri bu ortamda doğrulanamamıştır;
> bunlar **danışman gözetiminde elden kontrol edilmelidir**.

### Program bağımlılık grafiği

1. Ferrate, J., Ottenstein, T. ve Warren, J. (1988). *The Program Dependence Graph and Its Use in Automatic Software Deduction*. — Klasik kaynak; künye doğrulanmadı.
2. Weiss, D. J. (1991). *Program Dependence Graphs for the Detection of Harmful Interference*. — Klasik kaynak; künye doğrulanmadı.
3. Nagarajan, K., Bhargava, H. ve Shanbhag, J. (2009). *A Precise and Scalable Concurrent Program Dependence Graph*. — Klasik kaynak; künye doğrulanmadı.
4. Homerding, B., Patel, A. ve Deiana, E. A. (2024). *The Parallel Semantics Program Dependence Graph*. arXiv:2402.00986, 01.02.2024.
5. Askarunisa, A., Manju, T. ve Babu, G. G. (2012). *Fault Localization for Java Programs using Probabilistic Program Dependence Graph*. arXiv:1201.3985, 19.01.2012.
6. Zhang, Y., Li, G. ve Jin, Z. (2023). *Neural Program Repair with Program Dependence Analysis and Effective Filter*. arXiv:2305.09315, 16.05.2023.
7. Yan, Y., Cooper, N. ve Moran, K. (2026). *Enhancing Code Understanding for Impact Analysis by Combining Transformers*. arXiv:2607.23355, 25.07.2026.

### Çağrı grafiği

8. Ammons, M., Bodden, M. ve Hendren, L. (2002). *A Test-Generation Framework for Automatic Refactoring of Dynamic Access-Control Policies*. — Klasik kaynak; künye doğrulanmadı.
9. Bodden, M., Ammons, M. ve Hendren, L. (2010). *Reflection Analysis for Java*. — Klasik kaynak; künye doğrulanmadı.

### Gömme tabanlı kod araması

10. Wu, C. ve Yan, M. (2022). *Learning Deep Semantic Model for Code Search using CodeSearchNet Corpus*. arXiv:2201.11313, 27.01.2022.
11. Gu, J., Chen, Z. ve Monperrus, M. (2021). *Multimodal Representation for Neural Code Search*. arXiv:2107.00992, 02.07.2021.
12. Zeng, C., Yu, Y., Li, S., Zhang, X. ve Yang, Z. (2021). *deGraphCS: Embedding Variable-based Flow Graph for Neural Code Search*. arXiv:2103.13020, 24.03.2021.
13. Diera, A., Galke, L. ve Scherp, A. (2024). *Isotropy Matters: Soft-ZCA Whitening of Embeddings for Semantic Code Search*. arXiv:2411.17538, 26.11.2024.
14. Jain, S., Dora, A. ve Sam, K. S. (2024). *LLM Agents Improve Semantic Code Search*. arXiv:2408.11058, 05.08.2024.
15. Gandhi, S., Gao, L. ve Callan, J. (2025). *Repository-level Code Search with Neural Retrieval Methods*. arXiv:2502.07067, 10.02.2025.
16. Le-Anh, M., Nguyen, H., Tran, K. A., Le Hai, N., Van Linh, N., Bui, N. D. Q. ve Le, B. (2026). *Do Not Treat Code as Natural Language: Implications for Repository-Level Code Generation and Beyond*. arXiv:2602.11671, 12.02.2026.
17. Shah, P., Ghosh, R., Singhal, A. ve Dutta, D. (2025). *RANGER — Repository-Level Agent for Graph-Enhanced Retrieval*. arXiv:2509.25257, 27.09.2025.
18. Veenendaal, A. ve Daly, E. (2016). *Code Definition Analysis for Call Graph Generation*. arXiv:1610.04594, 27.07.2016.

### Servis bağımlılık grafiği

19. Neovius, M., Petre, L. ve Sere, K. (2016). *A Theory of Service Dependency*. arXiv:1606.02024, 07.06.2016.
20. Landau, D., Blanken, G. ve Barbosa, J. (2025). *Retrofitting Service Dependency Discovery in Distributed Systems*. arXiv:2510.15490, 17.10.2025.

### Nedensel çıkarım ve kök neden analizi

21. Zheng, Y., Huang, B. ve Chen, W. (2023). *Causal-learn: Causal Discovery in Python*. arXiv:2307.16405, 31.07.2023.
22. Ferdous, M. H., Hossain, E. ve Gani, M. O. (2025). *TimeGraph: Synthetic Benchmark Datasets for Robust Time-Series Causal Discovery*. arXiv:2506.01361, 02.06.2025.
23. Lin, C.-M., Chang, C. ve Wang, W.-Y. (2024). *Root Cause Analysis In Microservice Using Neural Granger Causal Discovery*. arXiv:2402.01140, 02.02.2024.
24. Pham, L., Ha, H. ve Zhang, H. (2024). *BARO: Robust Root Cause Analysis for Microservices via Multivariate Bayesian Online Causal Inference*. arXiv:2405.09330, 15.05.2024.
25. Pham, L., Ha, H. ve Zhang, H. (2024). *Root Cause Analysis for Microservice System based on Causal Inference: How Far Are We?* arXiv:2408.13729, 25.08.2024.
26. Zhang, S., Fang, A. ve Yang, Y. (2025). *DynaCausal: Dynamic Causality-Aware Root Cause Analysis for Distributed Microservices*. arXiv:2510.22613, 26.10.2025.
27. Wang, H., Wu, Z. ve Jiang, H. (2021). *Groot: An Event-graph-based Approach for Root Cause Analysis in Industrial Settings*. arXiv:2108.00344, 01.08.2021.
28. Zhao, Z., Wang, Z., Zhang, T. vd. (2024). *CHASE: A Causal Hypergraph based Framework for Root Cause Analysis in Multimodal Microservice Systems*. arXiv:2406.19711, 28.06.2024.
29. Xie, S., He, H. ve Wang, J. (2025). *Root Cause Analysis for Microservice Systems via Cascaded Conditional Learning*. arXiv:2511.17566, 14.11.2025.
30. Zhang, L., Jia, T. ve Zhai, Y. (2026). *Hypothesize-Then-Verify: Speculative Root Cause Analysis for Microservices*. arXiv:2601.02736, 06.01.2026.
31. Tang, P., Tang, S. ve Pu, H. (2025). *MicroRCA-Agent: Microservice Root Cause Analysis Method Based on Large Language Models*. arXiv:2509.15635, 19.09.2025.
32. Li, A. vd. (2018). *MicroScope: Online Failure Diagnosis for Microservice Applications*. — Klasik kaynak; künye doğrulanmadı.
33. Song, I. vd. (2014). *MicroRank: End-to-End Latency Issue Localization with Extended Spectrum Analysis in Microservice Environments*. — Klasik kaynak; künye doğrulanmadı.
34. Bi, T., Jiang, X. ve Zhang, X. (2026). *JustDiag!: A Diagnostic Justification Engine for Accountable Root Cause Analysis*. arXiv:2606.19407, 17.06.2026.

### Dil modeli ajanları ve araç zincirleri

35. Yao, S., Zhao, J., Yu, D. vd. (2022). *ReAct: Synergizing Reasoning and Acting in Language Models*. arXiv:2210.03629, 06.10.2022.
36. Cui, S., Krishna, R., Jha, S. ve Iyer, R. K. (2025). *PRAXIS: Integrating Program Analysis with Observability for Root-Cause Analysis*. arXiv:2512.22113, 26.12.2025.
37. Xu, H., Li, C. ve Ma, X. (2026). *The Evolution of Tool Use in LLM Agents: From Single-Tool Call to Multi-Tool*. arXiv:2603.22862, 24.03.2026.
38. Ehtesham, A., Singh, A., Gupta, G. K. ve Kumar, S. (2025). *A survey of agent interoperability protocols: Model Context Protocol (MCP), Agent Communication Protocol (ACP), Agent-to-Agent Protocol (A2A), and Agent Network Protocol (ANP)*. arXiv:2505.02279, 04.05.2025.
39. Luo, Z. vd. (2025). *MCP-Universe: Benchmarking Large Language Models with Real-World Model Context Protocol Servers*. arXiv:2508.14704, 20.08.2025.
40. Cho, M., Park, S., Jeong, K., Jeon, B., Choi, D. ve Han, J. (2026). *LLM within MCP Matters: Measuring Inefficient Resource Utilization Driven by LLMs*. arXiv:2608.08467, 09.08.2026.

### Diğer

41. Batista, C. P. ve Conan, D. (2026). *Spanergy: Energy-aware Distributed Tracing for Microservices*. arXiv:2607.24902, 27.07.2026.
42. OpenTelemetry. (t.y.). *OpenTelemetry Protocol Specification*. Erişim: 11.10.2026.
43. Mimar Sinan Güzel Sanatlar Üniversitesi. (2017). *Tez/Eser Metni Yazım Kılavuzu*.
44. Yükseköğretim Kurulu. (2024). *Yükseköğretimde Üretken Yapay Zekâ Kullanımına Dair Etik Rehber*.

---

**Gözlem.** Kaynakça 44 girdiden oluşmaktadır; bunların 37'si `paper` sunucusuyla
doğrulanmış, 7'si metin içinde adı geçen klasik kaynaklardır (1, 2, 3, 8, 9, 32,
33) ve künyeleri bu ortamda doğrulanamamıştır. Bu yedi girdi teslim öncesi elden
kontrol edilmelidir.

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