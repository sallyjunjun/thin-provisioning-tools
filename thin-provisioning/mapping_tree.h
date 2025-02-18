#ifndef MAPPING_TREE_H
#define MAPPING_TREE_H

#include "base/run.h"
#include "persistent-data/data-structures/btree.h"

//----------------------------------------------------------------

namespace thin_provisioning {
	namespace mapping_tree_detail {
		class space_map_ref_counter {
		public:
			space_map_ref_counter(space_map::ptr sm);

			void inc(block_address b);
			void dec(block_address b);

		private:
			space_map::ptr sm_;
		};

		struct block_time {
			uint64_t block_;
			uint32_t time_;
		};

		inline bool operator==(block_time const& lhs, block_time const& rhs) {
			return lhs.block_ == rhs.block_;
		}

		inline bool operator!=(block_time const& lhs, block_time const& rhs) {
			return !(lhs == rhs);
		}

		class block_time_ref_counter {
		public:
			block_time_ref_counter(space_map::ptr sm);
			void inc(block_time bt);
			void dec(block_time bt);

		private:
			space_map::ptr sm_;
		};

		struct block_traits {
			typedef base::le64 disk_type;
			typedef block_time value_type;
			typedef block_time_ref_counter ref_counter;

			static void unpack(disk_type const &disk, value_type &value);
			static void pack(value_type const &value, disk_type &disk);
		};

		class mtree_ref_counter final : public ref_counter<block_address> {
		public:
			mtree_ref_counter(transaction_manager &tm);

			void set(block_address const &b, uint32_t rc);
			void inc(block_address const &b);
			void dec(block_address const &b);

		private:
			transaction_manager &tm_;
		};

		// This value type is itself a tree containing mappings.
		// Used when manipulating the top level of the mapping
		// tree.
		struct mtree_traits {
			typedef base::le64 disk_type;
			typedef uint64_t value_type;
			typedef mtree_ref_counter ref_counter;

			static void unpack(disk_type const &disk, value_type &value);
			static void pack(value_type const &value, disk_type &disk);
		};

		//--------------------------------

		class damage_visitor;

		struct damage {
			virtual ~damage() {}
			virtual void visit(damage_visitor &v) const = 0;
		};

		struct missing_devices : public damage {
			missing_devices(std::string const &desc, run<uint64_t> const &keys);
			virtual void visit(damage_visitor &v) const;

			std::string desc_;
			run<uint64_t> keys_;
		};

		struct missing_mappings : public damage {
			missing_mappings(std::string const &desc, uint64_t thin_dev,
					 run<uint64_t> const &keys);
			virtual void visit(damage_visitor &v) const;

			std::string desc_;
			uint64_t thin_dev_;
			run<uint64_t> keys_;
		};

		class damage_visitor {
		public:
			typedef std::shared_ptr<damage_visitor> ptr;

			virtual ~damage_visitor() {}

			void visit(damage const &d) {
				d.visit(*this);
			}

			virtual void visit(missing_devices const &d) = 0;
			virtual void visit(missing_mappings const &d) = 0;
		};

		class mapping_visitor {
		public:
			virtual ~mapping_visitor() {}

			// path contains 2 elements, the dev key, then the oblock
			virtual void visit(btree_path const &path, block_time const &m) = 0;
		};

		class device_visitor {
		public:
			virtual ~device_visitor() {}
			virtual void visit(btree_path const &path, block_address dtree_root) = 0;
		};
	}

	typedef persistent_data::btree<2, mapping_tree_detail::block_traits> mapping_tree;
	typedef persistent_data::btree<1, mapping_tree_detail::mtree_traits> dev_tree;
	typedef persistent_data::btree<1, mapping_tree_detail::block_traits> single_mapping_tree;

	void walk_mapping_tree(dev_tree const &tree,
			       mapping_tree_detail::device_visitor &dev_v,
			       mapping_tree_detail::damage_visitor &dv,
			       bool ignore_non_fatal = false);

	void walk_mapping_tree(mapping_tree const &tree,
			       mapping_tree_detail::mapping_visitor &mv,
			       mapping_tree_detail::damage_visitor &dv,
			       bool ignore_non_fatal = false);

	void walk_mapping_tree(single_mapping_tree const &tree,
			       uint64_t dev_id,
			       mapping_tree_detail::mapping_visitor &mv,
			       mapping_tree_detail::damage_visitor &dv,
			       bool ignore_non_fatal = false);

	void check_mapping_tree(single_mapping_tree const &tree,
				uint64_t dev_id,
				mapping_tree_detail::damage_visitor &visitor,
				bool ignore_non_fatal = false);

	void check_mapping_tree(dev_tree const &tree,
				mapping_tree_detail::damage_visitor &visitor,
				bool ignore_non_fatal = false);

	void check_mapping_tree(mapping_tree const &tree,
				mapping_tree_detail::damage_visitor &visitor,
				bool ignore_non_fatal = false);
}

//----------------------------------------------------------------

#endif
