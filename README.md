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
4. 需要生成代码的关联关系，支持one-one和one-many，以及many-to-many模式，其中many-to-many的支持需要使用中间表。
5. 需要生成的Query的定义；

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
  allow-bool-widecard: true                                         # 生成出来的struct中的bool类型字段是否为宽松模式的解释，宽松模式是指，接受JSON中使用字符串为表达的bool值，如"true"或"false"，否则只能接受true或false
  allow-number-widecard: true                                       # 生成出来的struct中的number类型字段是否为宽松模式的解释，宽松模式是指，接受JSON中使用字符串为表达的number值，如"1.0"或"-20"，否则只能接受1.0, 20.0等。
  config-template-generate: D:/gitspaces/conf/rbatis.yml            # 是否对指定的yml文件进行相应的解释代码的生成，如果该yml所定义的文件比较完整，则生成出来的yml文件的解释程序也会是比较完整的。可以准确识别出多种数据类型以及结构
  api-handler-prefix: /api/v1                                       # 生成handler时URL的前缀
  webserver-port: "10899"                                           # 启动应用时服务器监听的端口，字符串类型，所以这里要用双引号
  schema-name: morinkhuur                                           # 数据库连接所对应的库名（或schema名称），这个字段必填，且应该与连接字的数据库或schema相同。
  tables:														    # 定义所有需要生成的表，可以定义多个
    - name: MORINKHUUR_USER                                         # 表名
      comment: 用户                                                  # 备注，对于该业务对象的备注，建议该字段都应该给出相应的代表准确含义的内容
      struct-name: MorinkhuurUser                                   # 生成出来的结构名称，如没有，则会根据name(表名)的PascalCase来自动产生。
      primary-key: user_id                                          # 定义主键，如果主键没有被定义，则会尝试从表结构来进行解析。
      all-field-option: true                                        # 是否所有的字段都是Option的。缺省为true。
      update-skip-fields: create_date                               # 执行更新操作时，需要进行跳过的字段。
      update-seletive: true                                         # 执行有选择地更新操作，该属性为true时，会生成一个update_selective方法，更新时，将只更新有值的字段
      page-query: true                                              # 是否生成分页查询
      default-sort-field: job_sort asc                              # 缺省的排序字段，如果有多个，可以以半角逗号隔开。所有生成出来的查询（列表和分页）都会加入这个缺省的排序。
      generate-param-struct: true                                   # 是否生成用于查询的结构，这个生成的用于查询的结构名称由实体的结构名+Query，如MorinkhuurUserQuery，生成出来的结构有以下两个特点：
                                                                    # 1. 日期/时间类型字段，生成出与实体中相对应的字段名一样的字段，但其类型为Vec<...>，如果在该Vec中不包含值，则该字段不参与查询，如果只包含一个值，
                                                                    # 则是按大于值进行查询，如果包含2个或以后，则是按第1个值和第2个值进行between .. and .. 查询。
                                                                    # 2. 如果字段名以_id, _code, _status, category, catalog, _type为结尾，则会在原基础上加多一个列表字段，如果列表字段不为空，则会对其执行IN (...) 查询。
      tree-parent-field: pid                                        # 是否支持有树形查询，且指定了用于树形查询的parent字段
      tree-root-value: "null"                                       # 表式为树形查询中为ROOT节点的数据条件，其值通常为"null"或为0，则会为其生成对应的查询语句，如 pid is null 或 pid = 0等。
      api-handler-name: user                                        # 生成actix-web的handler时，所对应的映射的url的主要部分，通常一个url为
             													    # ${api-handler-prefix}/${api-handler-name}/.. 组成，
             													    # api-handler-name中不要包含有非字母及数字之外的值，且不要以数字开头。
      simple-funclist:                    # 定义一些常用的简单的查询方法，通常在实际使用过程中，有些实体需要根据少数的一个或几个字段进行查询，我们就可以定义简单函数列表
      - func-name: load_username          # 定义函数的名称 
        condition: username               # 查询条件的字段
        list: false                       # 返回列表吗？ 
        self-func: false                  # 定义为需要引用self吗
        paged: false                      # 是否支持分页查询。 如果list以及paged都为false，则会生成返回单条记录的查询

  queries:														    # 定义自定义查询，对于一些复杂的SQL查询，可以定义query来实现
    - base-sql: SELECT * FROM MORINKHUUR_USER					    # 查询的基本sql，后面的功能会以该SQL作为基础来进行处理。
      struct-name: QueryUser                                        # 所生成的struct的名称
      generate-handler: true                                        # 是否生成actix-web的handler
      api-handler-name: user                                        # 生成actix-web的handler时，所对应的映射的url的主要部分，同table中的定义
      comment: 用户菜单查询                                          # 简短有效的备注 
      single-result: false                                          # 返回为单条记录
      params:                                                       # 固定的参数，固定参数是指被写在了base-sql中的必须指定的参数形态，以及理解为sql中有多少个问号，
      																                              # 就有多少个参数
        - column-names: id                                          # 参数的名称
          column-types: bigint                                      # 参数的类型
          default-value: 1                                          # 缺省值，用于rbatis-generator来执行base-sql，以便获得相应列信息。
      variant-params:												                        # 可变的参数，定义可变的参数可以在运行时根据传入的值来决定执行与之对应的条件
        - column-names: user_name                                   # 参数名称，如有多个时以半角逗号隔开
          column-types: varchar                                     # 参数类型，如有多个时以半角逗号隔开
          column-express: and MONITOR_USER = ?                      # 动态加入到SQL的表达式，如果表达式是多个条件，则可以一起列出。参数名称及类型也应该与之对应为多个。
  relations:                                                        # 定义一些需要按关系进行组合的实体
    - struct-name: ChimesUserRoles                                  # 对应的结构名称
      comment: 用户角色关系                                          # 简短有效的备注
      major-table: chimes_user                                      # 关联的主表
      extend-major: true                                            # 为true把主表的字段扩展开来，否则只是生成一个结构中一个字段
      generate-handler: true                                        # 是否生成actix-web的Handler
      generate-select: true                                         # 是否生成Select操作，实际是根据主键进行数据加载
      generate-save: true                                           # 是否生成保存操作，根据具体情况执行insert或update，
      generate-delete: true                                         # 是否生成删除操作，删除主表以及关联表的数据
      one-to-one:                                                   # 对于one-to-one的关联描述
        - table-name: chimes_profile                                # 所关联的表名
          join-field: user_id                                       # 关联表的关联字段
          major-field: user_id                                      # 主表的关联字段
      one-to-many:                                                  # 对于one-to-many的关联描述，用于描述one-to-many或many-to-many模式
        - table-name: chimes_role                                   # 关联表的表名 
          join-field: role_id                                       # 关联表的关联字段名
          major-field: user_id                                      # 主表的关联字段名
          middle-table: chimes_users_roles                          # 中间表名，当实际关系为many-to-many的情况下（如用户-角色的关系，实际为many-to-many模式），则需要使用中间表来进行连接，为准确描述many-to-many，实际上
                                                                    # 应该需要建立双边的one-to-many，如用户（1）--（*）角色的关系和角色（1）--（*）用户的关系。中间表在建立的时候，要满足一个要求，即中间表中的字段名应该与
                                                                    # 主表的关联字段名一致。如用户表的主键为user_id，角色表的主键为role_id，则中间表chimes_users_roles中字段应该为user_id和role_id。执行保存/删除时，
                                                                    # many-to-many关系下只会修改/删除中间表中的数据。
                                                                    # 如果中间表不存在，则表示主表与该表的关系是one-to-many，执行保存或删除操作时，会影响到关联表中的数据。

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

好了，大家可以下载它吧，然后开始你的rust之旅吧。

PS：目前只支持的MySQL数据库。其它的数据库支持后面再来加入了。

生成的代码目前使用了以下三个lib:
1. chimes-auth: 基于actix-web的授权的中间件；
2. chimes-rust: 基于actix-web的用户权限管理体系；
3. chimes-utils: 一些工具类以及函数，使用该lib，来获得 rbatis的数据库连接，可以支持多种数据库，它会将MySQL样式的SQL查询语句解析成对应的数据库，如PostgreSQL（目前只实现了该数据库的转换，对于MSSQL应该也是支持的，有兴趣的朋友可以测试一下）。

最近更新：
1、加入了上述在一个lib的引用；
2、使生成出来的代码没有编译错误且没有警告；
3、使生成出来的代码没有cargo clippy的警告，或尽可能少的警告。
4、建议在使用生成的代码前，执行一次cargo fmt。

