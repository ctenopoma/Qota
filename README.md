<div align="center">

<img src="src-tauri/icons/icon.png" alt="Qota Logo" width="80" height="80" />

# Qota

**シンプルで高速なローカル専用タスク管理アプリ**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Rust](https://img.shields.io/badge/Rust-stable-CE422B?logo=rust&logoColor=white)](https://www.rust-lang.org)

[インストール](#インストール) · [使い方](#使い方) · [開発環境のセットアップ](#開発環境のセットアップ) · [ドキュメント](#ドキュメント)

</div>

---

## ✨ 概要

**Qota** は、データをすべてローカルに保存するプライバシーファーストなタスク管理アプリです。クラウドへの接続は一切不要。あなたのタスクはあなたのマシン上だけに存在します。

Tauri v2 を採用しており、Rust バックエンドが高速・安全なデータ処理を担い、React + TypeScript のフロントエンドが快適な UI を提供します。

## 🚀 主な機能

| 機能 | 説明 |
|------|------|
| 📥 **受信箱 / 今日 / 今後** | GTD ワークフローに対応したビュー切り替え |
| 📋 **カンバンボード** | ドラッグ&ドロップでタスクを管理 |
| 🗂️ **プロジェクト管理** | タスクをプロジェクトにまとめて整理 |
| 🏷️ **タグ** | 柔軟なタグでクロスカテゴリ整理 |
| ⌨️ **コマンドパレット** | `Cmd/Ctrl+K` で素早くアクション実行 |
| 🔍 **全文検索** | `Cmd/Ctrl+/` でタスクを即座に検索 |
| 🌙 **ダーク / ライトモード** | 目に優しいテーマ切り替え |
| 🎨 **アクセントカラー** | 4 種類のカラーテーマから選択 |
| 📤 **JSON エクスポート / インポート** | データの可搬性を保証 |
| 🔒 **完全ローカル** | インターネット不要、プライバシー完全保護 |

### タスクの詳細機能

- ✅ サブタスク（ネスト対応）
- 🔴 優先度設定（高 / 中 / 低）
- 📅 期日・期限管理
- 📝 Markdown メモ
- 🏷️ 複数タグ付け

## 📦 インストール

### リリースビルドから（推奨）

> **Note**
> 現在 v0.1.0 開発中。リリースが利用可能になり次第ここに掲載します。

### ソースからビルド

[開発環境のセットアップ](#開発環境のセットアップ) を参照してください。

## 🖥️ 使い方

起動するとサイドバーから各ビューに切り替えできます。

```
受信箱     — 未分類の全タスク
今日       — 今日期限または「今日」フラグのタスク
今後       — カンバンボード（Todo / In Progress / Done）
プロジェクト — プロジェクト別タスク一覧
タグ       — タグ別タスク一覧
設定       — テーマ・アクセントカラー・データ管理
```

**ショートカット一覧**

| キー | アクション |
|------|-----------|
| `Ctrl/Cmd + K` | コマンドパレットを開く |
| `Ctrl/Cmd + /` | タスク検索 |
| `Esc` | モーダル / パレットを閉じる |
| `N` | 新規タスク作成（リスト表示時） |

## 🛠️ 開発環境のセットアップ

### 必要なツール

- **Node.js** 20 以上
- **npm** または **pnpm**
- **Rust** (最新 stable) — [rustup](https://rustup.rs/) でインストール
- **MSVC ビルドツール** (Windows のみ) — Visual Studio Build Tools 2022
- **Tauri CLI** v2

```bash
# Tauri CLI のインストール
cargo install tauri-cli --version "^2.0.0"
```

### クイックスタート

```bash
# 1. リポジトリのクローン
git clone https://github.com/your-username/qota.git
cd qota

# 2. フロントエンド依存のインストール
npm install

# 3. 開発サーバー起動（Tauri ウィンドウ）
npm run tauri dev
```

> **Note**
> フロントエンドのみ確認したい場合は `npm run dev`（http://localhost:1420）。
> ただし Rust バックエンドへの IPC が呼べないため、データ読み込みは動作しません。

### ビルド

```bash
# プロダクションビルド（Windows: MSI / NSIS インストーラを生成）
npm run tauri build
```

> **Warning**
> Windows では PowerShell の実行ポリシーを変更する必要がある場合があります。
> ```powershell
> Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
> ```

## 📁 プロジェクト構成

```
Qota/
├── src/                        # フロントエンド (React + TypeScript)
│   ├── components/             # 再利用可能 UI コンポーネント
│   ├── pages/                  # 各画面のページコンポーネント
│   ├── store/                  # Zustand グローバルステート
│   ├── lib/                    # ユーティリティ関数
│   ├── types.ts                # 共通型定義
│   ├── App.tsx                 # ルートコンポーネント
│   └── main.tsx                # エントリポイント
├── src-tauri/                  # バックエンド (Rust)
│   ├── src/
│   │   ├── main.rs             # Tauri アプリエントリ
│   │   ├── commands.rs         # IPC コマンドハンドラ
│   │   ├── models.rs           # データモデル
│   │   └── storage.rs          # JSON 永続化ロジック
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/
│   └── BACKEND.md              # Rust 実装の完全仕様書
└── public/                     # 静的アセット
```

## 💾 データ保存先

| OS | 保存パス |
|----|---------|
| Windows | `%APPDATA%\com.example.qota\qota.json` |
| macOS | `~/Library/Application Support/com.example.qota/qota.json` |
| Linux | `~/.local/share/com.example.qota/qota.json` |

データはすべてローカルの JSON ファイルに保存されます。定期的に **設定 → エクスポート** でバックアップすることをおすすめします。

## 📚 ドキュメント

- [バックエンド仕様書](docs/BACKEND.md) — Rust 実装の詳細（データスキーマ・IPC コマンド・エラー型）

## 🤝 コントリビュート

1. このリポジトリをフォーク
2. フィーチャーブランチを作成 (`git checkout -b feature/amazing-feature`)
3. 変更をコミット (`git commit -m 'Add amazing feature'`)
4. ブランチにプッシュ (`git push origin feature/amazing-feature`)
5. プルリクエストを作成

## 📄 ライセンス

[MIT License](LICENSE) © 2026 Qota Contributors
