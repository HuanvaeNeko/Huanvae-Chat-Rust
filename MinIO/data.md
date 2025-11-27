MinIO/
    data/

        avatars/        #存储用户头像                     （公开）

        group-file/     #群聊整体大文件夹                 （私密）
            group-id/   #以群聊id命名的文件夹方便定位     （私密）
                files/  #文档类型文件                     （私密）
                videos/ #视频类型文件                     （私密）
                images/ #图片类型文件                     （私密）

        user-file/      #个人用户整体大文件夹             （私密）
            user-id/    #以用户id命名的文件夹方便定位     （私密）
                files/  #文档类型文件                     （私密）
                videos/ #视频类型文件                     （私密）
                images/ #图片类型文件                     （私密）

        friends-file/               #以用户好友之间整体大文件夹                   （私密）
            {conversation-uuid}/    #以会话UUID命名的文件夹（用户ID排序组合） （私密）
                    files/          #文档类型文件                             （私密）
                    videos/         #视频类型文件                             （私密）
                    images/         #图片类型文件                             （私密）

公开为无需验证即可访问端点，私密为需要验证才可访问端点