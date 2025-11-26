#!/bin/bash

# テスト用の設定
TEST_DIR=$(mktemp -d)
SCRIPT_DIR="$(dirname "$0")"
SCRIPT_PATH="$SCRIPT_DIR/deny-check.sh"

# カラー定義
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

# カウンター
PASSED=0
FAILED=0

# テスト関数
test_case() {
    local name="$1"
    local input="$2"
    local expected_exit="$3"
    local settings="$4"
    
    # .claudeディレクトリを作成
    mkdir -p "$TEST_DIR/.claude"
    
    # 設定ファイルを作成
    echo "$settings" > "$TEST_DIR/.claude/settings.json"
    
    # テスト実行
    output=$(echo "$input" | HOME="$TEST_DIR" bash "$SCRIPT_PATH" 2>&1)
    actual_exit=$?
    
    if [ $actual_exit -eq $expected_exit ]; then
        echo -e "${GREEN}✓${NC} $name"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $name"
        echo "  期待: exit $expected_exit"
        echo "  実際: exit $actual_exit"
        echo "  出力: $output"
        ((FAILED++))
    fi
}

echo "=== deny-check.sh ユニットテスト ==="

# 基本機能テスト
test_case "Bashツール以外は許可" \
    '{"tool_name":"Read","tool_input":{"file_path":"/etc/passwd"}}' \
    0 \
    '{"permissions":{"deny":["Bash(rm -rf *)"]}}'

test_case "拒否パターンに完全一致" \
    '{"tool_name":"Bash","tool_input":{"command":"rm -rf /"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

test_case "拒否パターンに一致しない" \
    '{"tool_name":"Bash","tool_input":{"command":"ls -la"}}' \
    0 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

# ワイルドカードテスト
test_case "ワイルドカード*で拒否" \
    '{"tool_name":"Bash","tool_input":{"command":"rm -rf /home"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf *)"]}}'

# 複合コマンドテスト
test_case "セミコロン区切りで拒否" \
    '{"tool_name":"Bash","tool_input":{"command":"echo ok; rm -rf /"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

test_case "&&連結で拒否" \
    '{"tool_name":"Bash","tool_input":{"command":"cd /tmp && rm -rf /"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

test_case "||連結で拒否" \
    '{"tool_name":"Bash","tool_input":{"command":"false || rm -rf /"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

# パイプのテスト
test_case "パイプは分割されない" \
    '{"tool_name":"Bash","tool_input":{"command":"cat file | grep pattern"}}' \
    0 \
    '{"permissions":{"deny":["Bash(grep pattern)"]}}'

# 空白処理テスト
test_case "先頭空白を含むコマンド" \
    '{"tool_name":"Bash","tool_input":{"command":"  rm -rf /"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

test_case "末尾空白を含むコマンド" \
    '{"tool_name":"Bash","tool_input":{"command":"rm -rf /  "}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf /)"]}}'

# 複数パターンテスト
test_case "複数拒否パターンの1つにマッチ" \
    '{"tool_name":"Bash","tool_input":{"command":"sudo apt install vim"}}' \
    2 \
    '{"permissions":{"deny":["Bash(rm -rf *)", "Bash(sudo *)", "Bash(dd if=*)"]}}'

# エッジケース
test_case "空コマンド" \
    '{"tool_name":"Bash","tool_input":{"command":""}}' \
    0 \
    '{"permissions":{"deny":["Bash(rm -rf *)"]}}'

test_case "設定ファイルなし" \
    '{"tool_name":"Bash","tool_input":{"command":"ls"}}' \
    0 \
    ''

# クリーンアップ
rm -rf "$TEST_DIR"

# 結果表示
echo -e "\n結果: ${GREEN}$PASSED 成功${NC}, ${RED}$FAILED 失敗${NC}"
exit $((FAILED > 0 ? 1 : 0))