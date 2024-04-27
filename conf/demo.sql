-- aibulking.bulking_artifact definition

CREATE TABLE `bulking_artifact` (
  `artifact_id` bigint NOT NULL AUTO_INCREMENT COMMENT '表主键',
  `channel_id` bigint DEFAULT NULL COMMENT '关联所在渠道',
  `catalog_id` bigint DEFAULT NULL COMMENT '所在目录',
  `title` varchar(200) DEFAULT NULL COMMENT '作品标题',
  `author` varchar(100) DEFAULT NULL COMMENT '作者',
  `content` longtext COMMENT '内容',
  `compose_date` datetime(6) DEFAULT NULL COMMENT '作品的日期',
  `summary` text COMMENT '概要',
  `keywords` text COMMENT '关键词句',
  `create_date` datetime(6) DEFAULT NULL COMMENT '记录创建时间',
  `update_date` datetime(6) DEFAULT NULL COMMENT '修改时间',
  PRIMARY KEY (`artifact_id`),
  FULLTEXT KEY `bulking_artifact_title_IDX` (`title`,`content`,`summary`,`keywords`) /*!50100 WITH PARSER `ngram` */ 
) ENGINE=InnoDB AUTO_INCREMENT=1 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='文章，作品，内容表';


-- aibulking.bulking_artifact_training_materia definition

CREATE TABLE `bulking_artifact_training_materia` (
  `materia_id` bigint NOT NULL AUTO_INCREMENT,
  `artifact_id` bigint NOT NULL COMMENT '关联作品',
  `question` text COMMENT '训练问题',
  `answer` text COMMENT '训练回答',
  `effective` tinyint(1) DEFAULT NULL COMMENT '生效',
  `alpha_cost` double DEFAULT NULL COMMENT '训练成本',
  `create_date` datetime(6) DEFAULT NULL COMMENT '创建时间',
  `update_date` datetime(6) DEFAULT NULL COMMENT '更新时间',
  PRIMARY KEY (`materia_id`)
) ENGINE=InnoDB AUTO_INCREMENT=1 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='作品的训练材料';


-- aibulking.chimes_permission definition

CREATE TABLE `chimes_permission` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `alias` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci DEFAULT NULL,
  `create_time` datetime DEFAULT CURRENT_TIMESTAMP,
  `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci DEFAULT NULL,
  `pid` bigint NOT NULL,
  `api_pattern` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci DEFAULT NULL,
  `service_id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci DEFAULT NULL,
  `api_method` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci DEFAULT NULL,
  `api_bypass` varchar(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_permission_alias_t2` (`alias`) USING BTREE,
  KEY `idx_permission_name_service_id_t4` (`name`,`service_id`) USING BTREE,
  KEY `idx_permission_name_t2` (`name`) USING BTREE,
  KEY `idx_permission_service_id_t1` (`service_id`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=1 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;


-- aibulking.employee_shifts definition

CREATE TABLE `employee_shifts` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `work_date` date DEFAULT NULL COMMENT '日期',
  `employee_id` bigint DEFAULT NULL COMMENT '员工ID',
  `shift_id` bigint DEFAULT NULL COMMENT '班次ID，可为空',
  `shift_title` varchar(20) DEFAULT NULL COMMENT '该班描述，如果班次ID为空，则可以有，请假，出差，公共假等，否则，则应为与shift_id对应的班次名称',
  `remark` varchar(400) DEFAULT NULL COMMENT '备注描述',
  `create_time` datetime(6) DEFAULT NULL COMMENT '创建时间',
  `update_time` datetime(6) DEFAULT NULL COMMENT '修改时间',
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=308 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;


-- aibulking.shifts definition

CREATE TABLE `shifts` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `shift_name` varchar(20) DEFAULT NULL COMMENT '班次名称',
  `begin_time` varchar(20) DEFAULT NULL COMMENT '上班时间',
  `end_time` varchar(20) DEFAULT NULL COMMENT '下班时间',
  `default_shift` tinyint(1) DEFAULT NULL COMMENT '缺少班次，如果只有一个班次，则这个最好为True',
  `calc_overtime` tinyint(1) DEFAULT NULL COMMENT '超出时间算加班',
  `work_time` int DEFAULT NULL COMMENT '工作时间，一个班次的总的工作时间（按分钟计）',
  `break_time` int DEFAULT NULL COMMENT '中途可休息时间',
  `exceed` int DEFAULT NULL COMMENT '超出多少分钟开始计算加班',
  `create_time` datetime(6) DEFAULT NULL COMMENT '创建时间',
  `update_time` datetime(6) DEFAULT NULL COMMENT '修改时间',
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=1 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;


INSERT INTO shifts (id,shift_name,begin_time,end_time,default_shift,calc_overtime,work_time,break_time,exceed,create_time,update_time) 
VALUES (1,'白班','08:30','18:00',1,1,480,120,30,'2024-04-01 18:29:15','2024-04-11 16:29:41.443486');

INSERT INTO employee_shifts (id,work_date,employee_id,shift_id,shift_title,remark,create_time,update_time) 
VALUES (1,'2024-04-01',1,1,'白班','我们的假期不排班','2024-04-02 13:21:54.825565','2024-04-02 13:21:54.825570');
