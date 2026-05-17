作りたいものの整理
仮タイトル

Media Shelf

または日本語寄りなら、

私蔵メディア棚
TagShelf
Media Cabinet
視庫 / Shiko
蔵覧 / Zouran

あたりが合いそうです。

コンセプト

自分が持っている画像・動画を、
単なるフォルダ分けではなく、

人物
作品
ジャンル
場所
状態
用途
撮影日
お気に入り度
ネタ元
再視聴したい理由

などの意味付きタグで管理し、検索・閲覧・再生できるソフトウェア。

核になる考え方

普通のタグ管理はこうです。

猫
旅行
2024
ゲーム
お気に入り
資料

でも作りたいのはたぶんこう。

人物: 友人A
場所: 京都
種類: 旅行
状態: 未整理
用途: ブログ素材
評価: お気に入り
作品: シェンムー
キャラ: 芭月涼
媒体: 動画

つまり、タグにカテゴリー / 名前空間 / ファセットがある。

これはかなり重要です。

データモデル案
Media

画像・動画そのもの。

media
- id
- file_path
- file_type
- width
- height
- duration
- file_size
- hash
- created_at
- imported_at
- thumbnail_path
- metadata_json
TagCategory

タグの分類。

tag_categories
- id
- name
- color
- sort_order

例：

人物
場所
作品
ジャンル
状態
用途
イベント
評価
Tag

実際のタグ。

tags
- id
- category_id
- name
- description
- color

例：

人物: 友人A
場所: 秋葉原
作品: シェンムー
状態: 未整理
用途: 記事素材
評価: ★★★★★
MediaTag

メディアとタグの関連。

media_tags
- media_id
- tag_id
画面イメージ
1. ライブラリ画面

サムネイル一覧。

[検索バー]

カテゴリフィルタ:
人物: [友人A] [友人B]
場所: [東京] [京都] [横須賀]
作品: [シェンムー] [ポケカ] [自転車]
状態: [未整理] [整理済み] [要確認]

------------------------------------------------

[画像] [動画] [画像] [動画]
[画像] [画像] [動画] [画像]
2. 詳細画面

画像なら大きく表示。
動画なら再生。

--------------------------------
        メディア表示 / 動画再生
--------------------------------

ファイル名: IMG_20260516.mp4
種類: 動画
長さ: 00:03:21
解像度: 1920x1080

タグ:
人物: 友人A
場所: 横須賀
作品: シェンムー
用途: RTA調査
状態: 未整理

メモ:
この動画はイベント発火位置の確認用。
3. タグ編集画面
カテゴリ: 作品
タグ名: シェンムー

カテゴリ: 用途
タグ名: RTA資料

カテゴリ: 状態
タグ名: 要確認

ここで重要なのは、タグ入力時に

作品: シェンムー

みたいに補完できることです。

MVPとして作るなら

最初から動画解析やAIタグ付けまでやると重いです。
最初のMVPはこのくらいが良いです。

MVP 1
ローカルフォルダを指定して読み込み
画像・動画のサムネイル表示
画像表示
動画再生
タグカテゴリー作成
タグ作成
メディアにタグ付け
タグで絞り込み検索
SQLiteに保存

これだけでかなり使えます。

技術構成案

あなたの興味からすると、かなり相性がいいのはこれです。

Tauri + Svelte + SQLite
フロントエンド
Svelte / SvelteKit

理由：

軽い
ローカルアプリと相性がいい
UIを作りやすい
メディア一覧の状態管理が比較的素直
デスクトップアプリ
Tauri

理由：

ローカルファイル操作がしやすい
Electronより軽い
Rust側でファイルスキャンやサムネイル生成を任せられる
DB
SQLite

理由：

個人用ローカル管理にちょうどいい
タグ・検索・関連テーブルと相性がいい
バックアップしやすい
将来的に全文検索も使える
動画サムネイル
ffmpeg

または最初はブラウザ側の video 要素で簡易サムネイル生成でも良いです。

最初のフォルダ構成案
media-shelf/
  src/
    routes/
    lib/
      components/
        MediaGrid.svelte
        MediaViewer.svelte
        TagEditor.svelte
        TagFilterPanel.svelte
      stores/
        mediaStore.ts
        tagStore.ts

  src-tauri/
    src/
      main.rs
      media_scan.rs
      thumbnail.rs
      db.rs

  database/
    schema.sql
SQLiteスキーマ初期案
CREATE TABLE media (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_path TEXT NOT NULL UNIQUE,
  file_name TEXT NOT NULL,
  file_type TEXT NOT NULL,
  mime_type TEXT,
  width INTEGER,
  height INTEGER,
  duration_seconds REAL,
  file_size INTEGER,
  file_hash TEXT,
  thumbnail_path TEXT,
  imported_at TEXT NOT NULL,
  created_at TEXT,
  metadata_json TEXT
);

CREATE TABLE tag_categories (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  color TEXT,
  sort_order INTEGER DEFAULT 0
);

CREATE TABLE tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  category_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  color TEXT,
  FOREIGN KEY (category_id) REFERENCES tag_categories(id),
  UNIQUE(category_id, name)
);

CREATE TABLE media_tags (
  media_id INTEGER NOT NULL,
  tag_id INTEGER NOT NULL,
  PRIMARY KEY (media_id, tag_id),
  FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE,
  FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE TABLE notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  media_id INTEGER NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE
);
検索の思想

このソフトの面白いところは、単なる検索ではなく、

作品:シェンムー AND 用途:RTA資料 AND 状態:未整理

みたいな絞り込みができるところです。

さらに進めると、

作品:シェンムー
人物:芭月涼
場所:横須賀
種類:動画

のように、メディアを自分用の知識ベースとして扱える。

追加すると面白い機能
1. スマートコレクション

条件を保存する機能。

例：

未整理の動画
お気に入り画像
シェンムーRTA資料
ブログ素材候補
最近追加したもの
2. タグの親子関係
作品: ゲーム
  └ シェンムー
  └ エンドネシア
  └ ミズイロブラッド

これは後で追加でいいです。
最初からやると複雑になります。

3. 同一ファイル検出

ファイルハッシュで重複を見つける。

同じ画像が別フォルダにある
同じ動画を二重保存している

これも個人メディア管理ではかなり便利。

4. 自動タグ候補

最初はAIなしでもできます。

ファイルパスに "shenmue" がある → 作品:シェンムー
ファイルパスに "rta" がある → 用途:RTA資料
拡張子が mp4 → 種類:動画

後から画像認識やLLMを足せばいいです。

最初の開発順

おすすめはこれです。

1. SQLiteスキーマ作成
2. 指定フォルダをスキャン
3. media テーブルに登録
4. サムネイル一覧表示
5. 画像表示・動画再生
6. タグカテゴリー作成
7. タグ作成
8. メディアにタグ付け
9. タグフィルタ検索
10. スマートコレクション

ここまで行けば「使えるソフト」になります。

このソフトの本質

これはたぶん、単なるアルバムアプリではなくて、

自分が集めたメディアに意味を与える道具

です。

フォルダ管理は「置き場所」の管理ですが、
タグカテゴリー管理は「自分にとって何なのか」の管理です。

なので、あなた向けに言うならこれはかなり

メディア道普請ツール

っぽいです。

「画像・動画をしまう」のではなく、
「あとから辿れる道を敷く」ソフトですね。
