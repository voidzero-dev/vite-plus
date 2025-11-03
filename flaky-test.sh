set -e
for i in {1..100}; do
  echo "🔁 第 $i 次运行..."
  ls -al ~/Library/Caches/vite/package_manager
  rm -rf ~/Library/Caches/vite/package_manager
  RUST_BACKTRACE=full VITE_LOG=debug NPM_CONFIG_REGISTRY=https://registry.npmmirror.com cargo test
  #cargo test --package vite_install --lib -- package_manager
  # VITE_LOG=debug RUST_BACKTRACE=full RUST_LOG=debug NPM_CONFIG_REGISTRY=https://registry.npmmirror.com cargo test test_detect_package_manager_pnpmfile_over_yarn -- --nocapture
  ls -al ~/Library/Caches/vite/package_manager
done
echo "🎉 全部 10 次运行成功完成！"
