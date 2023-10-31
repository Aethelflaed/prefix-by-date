#!/usr/bin/env ruby

require 'fileutils'
require 'pathname'

if ARGV.length < 1
  puts "Please specify files"
  exit 1
end

def rename(old, new)
  puts "\tRenaming \"#{old}\" into \"#{new}\""
  old.rename(new)
end

PATTERN = /
  (?<rest>.+)
  \sau\s
  (?<year>\d{4})
  -
  (?<month>\d{2})
  -
  (?<day>\d{2})
  .pdf
/x

def process(filename)
  path = Pathname.new(filename)
  basename = path.basename.to_s

  if !path.exist?
    puts "#{path} does not exists"
    exit 2
  elsif match = PATTERN.match(basename)
    name = "#{match[:year]}-#{match[:month]}-#{match[:day]}"
    name = "#{name} #{match[:rest]}.pdf"

    rename(path, path.parent.join(name))
  else
    puts "Skipping #{path}"
  end
end

ARGV.each do |file|
  process file
end
