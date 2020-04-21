# frozen_string_literal: true

require 'rake'
require './rake_extensions.rb'

EXE_NAME = 'enforcer'
HOME = ENV['HOME']

task :default do
  puts 'no default task'
  create_changelog
end

desc 'Check'
task :check do
  sh 'cargo +nightly fmt -- --color=always --check'
  sh 'cargo clippy'
  sh 'cargo test'
end

desc 'run tests'
task :test do
  sh 'cargo test'
end

desc 'format code with cargo nightly'
task :fmt do
  sh 'cargo +nightly fmt'
end

desc 'run tests with printing to stdout'
task :test_nocapture do
  sh 'cargo test -- --nocapture'
end

def build_the_release
  sh 'cargo build --release'
  current_version = get_current_version
  release_folder = 'target/release'
  os_ext = 'darwin'
  if OS.linux?
    os_ext = 'linux'
  elsif OS.windows?
    os_ext = 'windows'
    release_folder = 'target/x86_64-pc-windows-gnu/release'
  end
  cd release_folder.to_s do
    cp EXE_NAME.to_s, "#{HOME}/bin/#{EXE_NAME}"
    sh "tar -cvzf #{EXE_NAME}@#{current_version}-#{os_ext}.tgz #{EXE_NAME}"
  end
end

def build_the_release_windows
  sh 'cargo build --release --target=x86_64-pc-windows-gnu'
  current_version = get_current_version
  release_folder = 'target/x86_64-pc-windows-gnu/release'
  tgz_file = "#{EXE_NAME}@#{current_version}-win64.tgz"
  cd release_folder.to_s do
    sh "tar -cvzf #{tgz_file} #{EXE_NAME}.exe"
  end
  mv "#{release_folder}/#{tgz_file}", 'target/release'
end

def build_the_release_windows32
  sh 'cargo build --release --target=i686-pc-windows-gnu'
  current_version = get_current_version
  release_folder = 'target/i686-pc-windows-gnu/release'
  tgz_file = "#{EXE_NAME}@#{current_version}-win32.tgz"
  cd release_folder.to_s do
    sh "tar -cvzf #{tgz_file} #{EXE_NAME}.exe"
  end
  mv "#{release_folder}/#{tgz_file}", 'target/release'
end

desc 'build release, no version bump'
task :build_release do
  build_the_release
  if OS.linux?
    build_the_release_windows
    build_the_release_windows32
  end
end

desc 'build release'
task :build do
  sh 'cargo build --release'
  pack_release
end

desc 'create new version and release'
task :create_release do
  current_tag = `git describe --tags`
  versioner = Versioner.for(:cargo_toml, '.')
  current_version = versioner.get_current_version
  unless current_tag.start_with?(current_version)
    raise "current tag #{current_tag} does not match current version: #{current_version}"
  end

  do_create_release(versioner)
end

def do_create_release(versioner)
  require 'highline'
  cli = HighLine.new
  cli.choose do |menu|
    default = :minor
    menu.prompt = "this will create and tag a new version (default: #{default}) "
    menu.choice(:minor) do
      create_and_tag_new_version(versioner, :minor)
    end
    menu.choice(:major) do
      create_and_tag_new_version(versioner, :major)
    end
    menu.choice(:patch) do
      create_and_tag_new_version(versioner, :patch)
    end
    menu.choice(:abort) { cli.say('ok...maybe later') }
    menu.default = default
  end
end

def create_and_tag_new_version(versioner, jump)
  current_version = versioner.get_current_version
  next_version = versioner.get_next_version(jump)
  assert_tag_exists(current_version)
  create_changelog(current_version, next_version)
  versioner.increment_version(jump)
  sh 'git add .'
  commit_cmd = "git commit -m \"Bump version from #{current_version} => #{next_version}\""
  tag_cmd = "git tag #{next_version}"
  puts "to commit, you can use ====> git add .; #{commit_cmd}"
  puts "to tag, use ===============> \"#{tag_cmd}\""
end

def pack_release
  require 'zip'
  exe_file = if OS.windows?
               "#{EXE_NAME}.exe"
             else
               EXE_NAME
             end
  exe_path = "target/release/#{exe_file}"

  zipfile_name = if OS.windows?
                   "#{EXE_NAME}_win.zip"
                 elsif OS.mac?
                   "#{EXE_NAME}_mac.zip"
                 else
                   "#{EXE_NAME}_linux.zip"
                 end
  rm_f zipfile_name

  Zip::File.open(zipfile_name, Zip::File::CREATE) do |zipfile|
    zipfile.add(exe_file, exe_path)
  end
end
