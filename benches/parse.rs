use std::time::Duration;

use criterion::{black_box, BenchmarkId, Criterion};
use mimalloc::MiMalloc;
use tmi::IrcMessageRef;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn twitch(c: &mut Criterion) {
  // messages representative of all of twitch dot television:
  let long_line = "@badge-info=;badges=;color=#008000;display-name=Allister20;emote-only=1;emotes=emotesv2_36040ff90da142938fa53287cf166373:328-334/emotesv2_e88cc46144b84732929c75512e8a2d3d:399-407/308078032:6-14/emotesv2_5d509428f6c748be8c597e9c416957b7:34-41/emotesv2_f018575b018f4c8f89a9076c7c3a47ac:70-77/emotesv2_354e23a6676f4576b51939c4b959ac33:292-298/emotesv2_a61b82c209db409db562409d5bc598c1:320-326/emotesv2_025b5879f91848d9920800592d1cb611:344-351/emotesv2_141f5da3271a4d958d684469d988365d:91-99/emotesv2_fdc6fc40fc3f4c0d8a43fedbfbd8204b:191-198/emotesv2_03a37edcf7554c968c1c10064799b0df:211-217/emotesv2_4d154487ee3844adbb724993bd905684:248-255/emotesv2_af82b76c0bfa42cabe9224690f723a4c:336-342/emotesv2_5dd1d324b05848399f6de05e409b7513:300-308/emotesv2_f912884613514b5c9a80bff390e1b5ca:381-388/emotesv2_0eb8bac56ab5443bb854d2335a12b47c:409-417/emotesv2_e9a734af13324b46bcd69bc1451251b2:43-54/304482946:157-163/emotesv2_51a4ee0cfe764f8db60cedb7168f361e:219-226/emotesv2_04ff32c657d149a5bb4bc73b38a87d3e:266-274/emotesv2_6d4b9357d5764b268ea5d2aea852fde7:276-282/emotesv2_f5518bacd19c432f872f90e5262fd2f0:0-4/emotesv2_af79a4289df346d88e8083357189662a:79-89/emotesv2_11eff6a54749464c9dfa40570dd356bd:101-108/emotesv2_3b57922dc1464a55870170158f97e1a8:165-171/emotesv2_3c1551a89eda4214b05bf60fcacac9d0:362-370/emotesv2_f366652225ca4f21bc0d3bd1fa790965:141-147/emotesv2_34b2a285e0ba49808daa933f7b143056:200-209/306112344:238-246/emotesv2_2589054b7fb64622b983367b99a65d01:257-264/emotesv2_be17775b4c9d4b3ab62ca6beeb098eee:353-360/emotesv2_c013e62f7945411da8d0fb7a03dd5e4f:182-189/emotesv2_79090f64904d4d499091ad71662cd60f:228-236/emotesv2_67cfc3d84f244644a6891e57215cf79d:419-428/emotesv2_6b0dac6e57584f84a0cc903b6afac595:56-62/emotesv2_f6d589cb12cd4f9fb9fb7741277464b1:110-117/emotesv2_7e50cd0fb15c4e26a8d8f05dbfc68aa8:119-128/emotesv2_3f8df5684f254ab99725dde938ce39b2:149-155/emotesv2_5278f8ea850942ebaf9efb88f9a16e4d:173-180/emotesv2_4397a8b926944ee19e39528decc7a23b:16-23/emotesv2_a820613d7b86414385f13f715dc4c3b3:64-68/emotesv2_fb857e88c017438ebdb28806ea50db49:284-290/emotesv2_e50c94d7ab144c8fbc70abdbd41653dd:430-437/emotesv2_f4226bd2c0334cd290ea2b064b76ef54:439-445/emotesv2_ccad3c03685a4c90bc5c2cac375e0264:25-32/emotesv2_28c5dbd1746e49bb9d97b94d0d151f4e:130-139/emotesv2_214c32a3dcd2419abac72d0c6959fd62:310-318/emotesv2_b64198cde643471cbf3a48c6346e1643:372-379/emotesv2_71685e25a9b74ed1832388b1a2e39800:390-397;first-msg=0;flags=;id=73fce4cb-b215-4bc6-acfc-caa4efd7c381;mod=0;returning-chatter=0;room-id=22484632;subscriber=0;tmi-sent-ts=1685734498590;turbo=0;user-id=169252202;user-type= :allister20!allister20@allister20.tmi.twitch.tv PRIVMSG #forsen :elis7 elisBased elisBite elisBlob elisBruh elisBusiness elisCry elisD elisDank elisDespair elisEHEHE elisElis elisFail elisFlower elisGrumpy elisHmm elisHug elisHuh elisNom elisNerd elisLove elisLost elisLookUp elisLUL elisIsee elisICANT elisOmega elisPain elisPray elisShrug elisShy elisSip elisSit elisSleep elisSmile elisYes elisWow elisWot elisWave elisUWAA elisSweat elisSubs elisSmug elisSlap elisDance elisDancy elisRockin elisSpin elisYay";
  let average_line = "@badge-info=subscriber/30;badges=subscriber/24;client-nonce=0cbb6912300538decb76d5d64f7a6e60;color=#0000FF;display-name=TheShiz93;emotes=;first-msg=0;flags=;id=14aa5932-d95c-430a-9a68-a62cc9310f58;mod=0;returning-chatter=0;room-id=22484632;subscriber=1;tmi-sent-ts=1685665726948;turbo=0;user-id=48356725;user-type= :theshiz93!theshiz93@theshiz93.tmi.twitch.tv PRIVMSG #forsen :!ecount Ogey";
  let short_line = "@room-id=22484632;target-user-id=916768740;tmi-sent-ts=1685713312412 :tmi.twitch.tv CLEARCHAT #forsen :forsenclone666";

  let mut bench = |name: &str, line: &str| {
    c.bench_with_input(BenchmarkId::new("twitch", name), &line, |b, line| {
      b.iter(|| {
        black_box(IrcMessageRef::parse(line).expect("failed to parse"));
      });
    });
  };

  bench("long", long_line);
  bench("average", average_line);
  bench("short", short_line);
}

fn criterion() -> Criterion {
  Criterion::default()
    .configure_from_args()
    .warm_up_time(Duration::from_millis(100))
    .measurement_time(Duration::from_secs(1))
    .sample_size(1000)
}

fn main() {
  let mut criterion = criterion();
  twitch(&mut criterion);
  criterion.final_summary();
}
