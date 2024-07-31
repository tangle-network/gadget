import os
import zipfile
import argparse

def zip_files_with_extension(input_dir, file_extension, output_zip):
    with zipfile.ZipFile(output_zip, 'w', zipfile.ZIP_DEFLATED) as zipf:
        for root, _, files in os.walk(input_dir):
            for file in files:
                if file.endswith(file_extension):
                    file_path = os.path.join(root, file)
                    zipf.write(file_path, os.path.relpath(file_path, input_dir))

def main():
    parser = argparse.ArgumentParser(description='Zip files with a specified extension recursively.')
    parser.add_argument('input_dir', metavar='input_dir', type=str, help='Input directory path')
    parser.add_argument('file_extension', metavar='file_extension', type=str, help='File extension (e.g., ".rs")')
    parser.add_argument('-o', '--output', metavar='output_zip', type=str, default='output.zip', help='Output zip file name (default: output.zip)')

    args = parser.parse_args()

    input_directory = args.input_dir
    file_ext = args.file_extension
    output_zip_file = args.output

    if not file_ext.startswith('.'):
        file_ext = '.' + file_ext

    zip_files_with_extension(input_directory, file_ext, output_zip_file)
    print(f'Files with extension "{file_ext}" have been zipped to "{output_zip_file}".')

if __name__ == '__main__':
    main()