# rbatis-generator

#### 介绍
Rust语言生成采用rbatis和actix-web的源码的工具。
该工具使用rbatis (基于sqlx为基础实现的类似于MyBatis的ORM工具)。同时，该工具还可以生成相应的基于actix-web的路由实现代码。
它可以说是一个面向初学者友好的工具。它所生成的代码可以帮助Rust初学者快速理解rbatis的工作过程和模式，以及actix-web的工作模式。

rbatis-generator采用配置文件rbatis.yml来管理将被生成的项目的内容。
配置文件主要有：
1. 数据库连接；
2. 代码生成的项目属性；
3. 需要生成代码的Table的定义；
4. 需要生成的Query的定义；

下面就rbatis.yml进行详细说明：

-->1. 首先是数据库连接字的配置：
```
database:
  url: mysql://chimes:ks123456@localhost:3306/morinkhuur
```
-->2. 生成出来的应该信息
```
codegen:
  app-name: MorinkhuurCanary										# 应用名称
  app-authors: Long(long.zou@gmail.com)								# 应用作者
  app-edition: "2021"												# Rust Edition
  app-version: 0.1.0    											# 应用版本号
  output-path: d:/temp/rust/									    # 代码生成后所输出的目录
  always-generate-handler: true                                     # 是否生成actix-web的handler代码
  always-generate-entity: true                                      # 是否生成entity的代码，包括struct定义以及常的curd方法
  api-handler-prefix: /api/v1                                       # 生成handler时URL的前缀
  webserver-port: "10899"                                           # 启动应用时服务器监听的端口，字符串类型，所以这里要用双引号
  schema-name: morinkhuur                                           # 数据库连接所对应的库名（或schema名称），这个字段必填，且应该与连接字的数据库或schema相同。
  tables:														    # 定义所有需要生成的表，可以定义多个
    - name: MORINKHUUR_USER                                         # 表名
      struct-name: MorinkhuurUser                                   # 生成出来的结构名称，如没有，则会根据name(表名)的PascalCase来自动产生。
      primary-key: user_id                                          # 定义主键，如果主键没有被定义，则会尝试从表结构来进行解析。
      all-field-option: true                                        # 是否所有的字段都是Option的。缺省为true。
      update-skip-fields: create_date                               # 执行更新操作时，需要进行跳过的字段。
      update-seletive: true                                         # 执行有选择地更新操作，该属性为true时，会生成一个update_selective方法，更新时，将只更新有值的字段
      page-query: true                                              # 是否生成分页查询
      api-handler-name: user                                        # 生成actix-web的handler时，所对应的映射的url的主要部分，通常一个url为
             													    # ${api-handler-prefix}/${api-handler-name}/.. 组成，
             													    # api-handler-name中不要包含有非字母及数字之外的值，且不要以数字开头。

  queries:														    # 定义自定义查询，对于一些复杂的SQL查询，可以定义query来实现
    - base-sql: SELECT * FROM MORINKHUUR_USER					    # 查询的基本sql，后面的功能会以该SQL作为基础来进行处理。
      struct-name: QueryUser                                        # 所生成的struct的名称
      generate-handler: true                                        # 是否生成actix-web的handler
      api-handler-name: user                                        # 生成actix-web的handler时，所对应的映射的url的主要部分，同table中的定义
      params:                                                       # 固定的参数，固定参数是指被写在了base-sql中的必须指定的参数形态，以及理解为sql中有多少个问号，
      																# 就有多少个参数
        - column-names: id                                          # 参数的名称
          column-types: bigint                                      # 参数的类型
          default-value: 1                                          # 缺省值，用于rbatis-generator来执行base-sql，以便获得相应列信息。
      variant-params:												# 可变的参数，定义可变的参数可以在运行时根据传入的值来决定执行与之对应的条件
        - column-names: user_name                                   # 参数名称，如有多个时以半角逗号隔开
          column-types: varchar                                     # 参数类型，如有多个时以半角逗号隔开
          column-express: and MONITOR_USER = ?                      # 动态加入到SQL的表达式，如果表达式是多个条件，则可以一起列出。参数名称及类型也应该与之对应为多个。
```

#### 安装及使用说明

1.  下载
2.  根据你的需要修改 conf/rbatis.yml
3.  执行 cargo b 或者 cargo b -r
4.  执行 target/debug/rbatisgen 或 target/release/rbatisgen
5.  找到输出目录，所生成的rust代码就在该目录下。
通常，输出的rust代码有如下的目录结构：
#### 
```
1. |--src
2. |  |--conf   (存放配置文件的目录)
3. |  |--entity (存放所有的基于表的代码生成)
4. |  |--handler （存放所生成的actix-web的handler代码
5. |  |--query (如果rbatis.yml定义了自定义查询则存在)
6. |  |--utils （一些工具函数或类）
7. |  |--main.rs （主文件）
8. |--Cargo.toml 
```

好了，大家无情地下载它吧，然后开始你的rust之旅吧。

PS：目前只支持的MySQL数据库。其它的数据库支持后面再来加入了。



# 联系方式/捐赠,或 [rbatis-generator](https://github.com/longzou/rbatis-generator) 点star

> 捐赠

<img style="width: 400px;height: 600px;" width="400" height="600" src="https://gitee.com/poethxp/rbatis-generator/raw/master/wx_account.jpg" alt="enjoylost" />

