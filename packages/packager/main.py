#!/usr/bin/env python3
import os
import argparse
import gzip
import json
import os
import struct
import time

from resource import build as build_resource

# def update_exe_resources(input_exe, resource_data, meta, output_exe):
#     """
#     采用 Windows API 更新资源的方式将 RESOURCE_DATA 和 RESOURCE_META 嵌入到 exe 中，
#     资源类型使用 RT_RCDATA，资源名称分别为 "RESOURCE_DATA" 和 "RESOURCE_META"。

#     操作流程：
#       1. 将输入 exe 拷贝到输出 exe（因为资源更新操作是就地修改）。
#       2. 调用 BeginUpdateResource 打开输出 exe 进行资源更新。
#       3. 分别调用 UpdateResource 添加两个资源。
#       4. 调用 EndUpdateResource 提交更新。
#     """
#     # 复制输入 exe 到输出 exe
#     shutil.copy2(input_exe, output_exe)
#     # 将 meta 信息转换为 JSON 字符串后编码为字节数据
#     meta_bytes = json.dumps(meta, indent=4).encode("utf-8")
#     # 开始资源更新，第二个参数 False 表示不删除原有的资源
#     hUpdate = win32api.BeginUpdateResource(output_exe, False)
#     try:
#         # RT_RCDATA 对应的常量为 win32con.RT_RCDATA
#         win32api.UpdateResource(
#             hUpdate, win32con.RT_RCDATA, "RESOURCE_DATA", bytes(resource_data))
#         win32api.UpdateResource(
#             hUpdate, win32con.RT_RCDATA, "RESOURCE_META", meta_bytes)
#     except Exception as e:
#         # 出现异常时放弃修改，恢复 exe 原状态
#         win32api.EndUpdateResource(hUpdate, True)
#         raise e
#     # 提交资源更新
#     win32api.EndUpdateResource(hUpdate, False)


def main():
    parser = argparse.ArgumentParser(
        description="将资源目录打包为 Windows 资源，并嵌入到可执行文件中")
    parser.add_argument("--gzip", action="store_true",
                        help="对整个资源数据使用 gzip 压缩")
    parser.add_argument("resource_dir", help="资源目录路径")
    # parser.add_argument("input_exe", help="待打包的 Windows 可执行文件")
    # parser.add_argument("output_exe", help="打包后的 Windows 可执行文件")
    args = parser.parse_args()

    resource_dir = os.path.abspath(args.resource_dir)
    print("resource_dir: ", resource_dir)
    resource = build_resource(resource_dir, args.gzip)
    with open("resource.bin", "wb") as f:
        f.write(resource)
    print("打包完成，生成的资源文件：", "resource.bin")

    # # 生成资源数据和 meta 信息
    # resource_data, meta = create_resource_files(args.resource_dir, args.gzip)
    # write_resource_files(resource_data, meta)
    # # 更新 exe 内部的资源表，将数据以 RT_RCDATA 资源嵌入进去
    # input_exe = os.path.abspath(args.input_exe)
    # output_exe = os.path.abspath(args.output_exe)
    # update_exe_resources(input_exe, resource_data, meta, output_exe)
    # print("打包完成，生成的可执行文件：", args.output_exe)

if __name__ == "__main__":
    main()
